use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::classfile::ClassFile;
use crate::relocator::Relocator;
use crate::treeshake::TreeShaker;

pub struct JarProcessor<'a> {
    pub relocator: &'a Relocator,
    pub tree_shake: bool,
    pub custom_entry_points: Vec<String>,
    pub keep_prefixes: Vec<String>,
    pub strip_multi_release: bool,
    pub minimize_resources: bool,
}

impl<'a> JarProcessor<'a> {
    pub fn new(
        relocator: &'a Relocator,
        tree_shake: bool,
        custom_entry_points: Vec<String>,
        keep_prefixes: Vec<String>,
        strip_multi_release: bool,
        minimize_resources: bool,
    ) -> Self {
        Self {
            relocator,
            tree_shake,
            custom_entry_points,
            keep_prefixes,
            strip_multi_release,
            minimize_resources,
        }
    }

    /// Проверка, является ли файл мусорной подписью безопасности JAR или мусорным ресурсом
    fn is_signature_or_junk(path: &str, strip_mr: bool, minimize: bool) -> bool {
        let p = path.to_ascii_uppercase();

        // Multi-Release JAR stripping: META-INF/versions/9/..., META-INF/versions/11/...
        if strip_mr && p.starts_with("META-INF/VERSIONS/") {
            return true;
        }

        // Мусорные лицензии, README, дубликаты module-info при минимизации
        if minimize {
            if p == "MODULE-INFO.CLASS" || p.ends_with("/MODULE-INFO.CLASS") {
                return true;
            }
            if p.starts_with("META-INF/") {
                if p.ends_with(".TXT")
                    || p.ends_with(".MD")
                    || p.contains("LICENSE")
                    || p.contains("NOTICE")
                    || p.contains("COPYRIGHT")
                {
                    return true;
                }
            }
        }

        if p.starts_with("META-INF/") {
            if p.ends_with(".SF") || p.ends_with(".DSA") || p.ends_with(".RSA") || p.starts_with("META-INF/SIG-") {
                return true;
            }
            if p.starts_with("META-INF/MAVEN/") {
                return true;
            }
        }
        false
    }

    pub fn process<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> Result<(), Box<dyn std::error::Error>> {
        let in_file = File::open(input_path)?;
        let mut archive = ZipArchive::new(in_file)?;

        let mut entry_points = HashSet::new();
        for ep in &self.custom_entry_points {
            entry_points.insert(ep.clone());
        }

        let mut service_providers = HashSet::new();
        let mut service_providers_merged: HashMap<String, HashSet<String>> = HashMap::new();
        let mut class_map: HashMap<String, ClassFile> = HashMap::new();
        let mut raw_files: HashMap<String, Vec<u8>> = HashMap::new();

        let num_files = archive.len();
        for i in 0..num_files {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();

            if file.is_dir() || Self::is_signature_or_junk(&file_name, self.strip_multi_release, self.minimize_resources) {
                continue;
            }

            let mut buffer = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buffer)?;

            if file_name == "META-INF/MANIFEST.MF" {
                if let Ok(manifest_text) = std::str::from_utf8(&buffer) {
                    if let Some(main) = TreeShaker::extract_manifest_main_class(manifest_text) {
                        entry_points.insert(main);
                    }
                }
                raw_files.insert(file_name, buffer);
            } else if file_name == "plugin.yml" || file_name == "paper-plugin.yml" {
                if let Ok(plugin_text) = std::str::from_utf8(&buffer) {
                    for main in TreeShaker::extract_plugin_yml_main(plugin_text) {
                        entry_points.insert(main);
                    }
                    // Умная авто-релокация полей внутри манифеста плагина
                    let relocated_yaml = TreeShaker::relocate_plugin_yml(plugin_text, self.relocator);
                    raw_files.insert(file_name, relocated_yaml.into_bytes());
                } else {
                    raw_files.insert(file_name, buffer);
                }
            } else if file_name.starts_with("META-INF/services/") {
                if let Ok(text) = std::str::from_utf8(&buffer) {
                    for cls in TreeShaker::extract_services_classes(text) {
                        service_providers.insert(cls.clone());
                        service_providers_merged
                            .entry(file_name.clone())
                            .or_default()
                            .insert(cls);
                    }
                }
            } else if file_name.ends_with(".class") {
                if let Ok(cf) = ClassFile::parse(&buffer) {
                    let internal_name = file_name.trim_end_matches(".class").to_string();
                    class_map.insert(internal_name, cf);
                } else {
                    raw_files.insert(file_name, buffer);
                }
            } else {
                raw_files.insert(file_name, buffer);
            }
        }

        // Шаг 2: Tree-shaking (все неиспользуемые библиотеки)
        let (reachable_opt, project_classes_opt) = if self.tree_shake {
            let shaker = TreeShaker::new(entry_points, self.keep_prefixes.clone(), service_providers);
            let (reach, proj) = shaker.compute_reachable(&class_map);
            (Some(reach), Some(proj))
        } else {
            (None, None)
        };

        // Шаг 3: Запись оптимизированного JAR
        let out_file = File::create(output_path)?;
        let mut writer = ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut added_dirs = HashSet::new();

        // 3.1. Запись живых классов
        for (class_name, mut cf) in class_map {
            if let Some(ref reach) = reachable_opt {
                let is_project = project_classes_opt
                    .as_ref()
                    .map(|p| p.contains(&class_name))
                    .unwrap_or(false);

                // Если класс не в проекте и не достижим в графе вызовов -> ВЫРЕЗАЕМ!
                if !is_project && !reach.contains(&class_name) {
                    continue;
                }
            }

            // Применяем релокацию constant pool
            self.relocator.relocate_class(&mut cf);

            // Релоцируем имя файла
            let relocated_class_name = self.relocator.relocate_path(&class_name);
            let relocated_file_name = format!("{}.class", relocated_class_name);

            // Обеспечиваем существование родительских директорий в ZIP
            if let Some(parent) = Path::new(&relocated_file_name).parent() {
                let parent_str = parent.to_string_lossy().replace('\\', "/");
                if !parent_str.is_empty() && added_dirs.insert(parent_str.clone()) {
                    let dir_name = format!("{}/", parent_str);
                    writer.add_directory(dir_name, SimpleFileOptions::default())?;
                }
            }

            let class_bytes = cf.write_to_bytes()?;
            writer.start_file(relocated_file_name, options)?;
            writer.write_all(&class_bytes)?;
        }

        // 3.2. Запись объединенных META-INF/services/ с дедупликацией и релокацией
        for (file_name, impls) in service_providers_merged {
            let (new_file_name, _, _) = self.relocator.relocate_service_resource(&file_name, "");

            let mut out_impls: Vec<String> = Vec::new();
            for impl_slash in impls {
                if let Some(ref reach) = reachable_opt {
                    if !reach.contains(&impl_slash) {
                        continue;
                    }
                }
                let relocated_slash = self.relocator.relocate_path(&impl_slash);
                let relocated_dot = relocated_slash.replace('/', ".");
                if !out_impls.contains(&relocated_dot) {
                    out_impls.push(relocated_dot);
                }
            }

            if !out_impls.is_empty() {
                out_impls.sort();
                writer.start_file(new_file_name, options)?;
                writer.write_all(out_impls.join("\n").as_bytes())?;
                writer.write_all(b"\n")?;
            }
        }

        // 3.3. Запись остальных ресурсов
        for (file_name, buffer) in raw_files {
            // Обычный ресурс
            let relocated_file_name = self.relocator.relocate_path(&file_name);
            writer.start_file(relocated_file_name, options)?;
            writer.write_all(&buffer)?;
        }

        writer.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_and_junk_detection() {
        assert!(JarProcessor::is_signature_or_junk("META-INF/CODESIGN.SF", false, false));
        assert!(JarProcessor::is_signature_or_junk("META-INF/CODESIGN.RSA", false, false));
        assert!(JarProcessor::is_signature_or_junk("META-INF/maven/pom.xml", false, false));

        // Multi-release stripping
        assert!(!JarProcessor::is_signature_or_junk("META-INF/versions/9/module-info.class", false, false));
        assert!(JarProcessor::is_signature_or_junk("META-INF/versions/9/module-info.class", true, false));
        assert!(JarProcessor::is_signature_or_junk("META-INF/versions/17/com/example/Helper.class", true, false));

        // Resource minimization
        assert!(!JarProcessor::is_signature_or_junk("META-INF/LICENSE.txt", false, false));
        assert!(JarProcessor::is_signature_or_junk("META-INF/LICENSE.txt", false, true));
        assert!(JarProcessor::is_signature_or_junk("META-INF/NOTICE", false, true));
        assert!(JarProcessor::is_signature_or_junk("module-info.class", false, true));
        assert!(!JarProcessor::is_signature_or_junk("com/example/MyClass.class", true, true));
    }
}

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
}

impl<'a> JarProcessor<'a> {
    pub fn new(
        relocator: &'a Relocator,
        tree_shake: bool,
        custom_entry_points: Vec<String>,
        keep_prefixes: Vec<String>,
    ) -> Self {
        Self {
            relocator,
            tree_shake,
            custom_entry_points,
            keep_prefixes,
        }
    }

    /// Проверка, является ли файл мусорной подписью безопасности JAR
    fn is_signature_or_junk(path: &str) -> bool {
        let p = path.to_ascii_uppercase();
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
        let mut class_map: HashMap<String, ClassFile> = HashMap::new();
        let mut raw_files: HashMap<String, Vec<u8>> = HashMap::new();

        let num_files = archive.len();
        for i in 0..num_files {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();

            if file.is_dir() || Self::is_signature_or_junk(&file_name) {
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
            } else if file_name == "plugin.yml" {
                if let Ok(plugin_text) = std::str::from_utf8(&buffer) {
                    for main in TreeShaker::extract_plugin_yml_main(plugin_text) {
                        entry_points.insert(main);
                    }
                }
                raw_files.insert(file_name, buffer);
            } else if file_name.starts_with("META-INF/services/") {
                if let Ok(text) = std::str::from_utf8(&buffer) {
                    for cls in TreeShaker::extract_services_classes(text) {
                        service_providers.insert(cls);
                    }
                }
                raw_files.insert(file_name, buffer);
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

        // 3.2. Запись ресурсов и META-INF/services/
        for (file_name, buffer) in raw_files {
            if file_name.starts_with("META-INF/services/") {
                if let Ok(text) = std::str::from_utf8(&buffer) {
                    let (new_file_name, new_content, _) =
                        self.relocator.relocate_service_resource(&file_name, text);

                    // Оставляем только те строки реализаций, которые живы (если включен tree-shaking)
                    let filtered_lines: Vec<&str> = if let Some(ref reach) = reachable_opt {
                        new_content
                            .lines()
                            .filter(|line| {
                                let trimmed = line.trim();
                                if trimmed.starts_with('#') || trimmed.is_empty() {
                                    return true;
                                }
                                let slash = trimmed.replace('.', "/");
                                reach.contains(&slash)
                            })
                            .collect()
                    } else {
                        new_content.lines().collect()
                    };

                    if !filtered_lines.is_empty() {
                        writer.start_file(new_file_name, options)?;
                        writer.write_all(filtered_lines.join("\n").as_bytes())?;
                    }
                    continue;
                }
            }

            // Обычный ресурс
            let relocated_file_name = self.relocator.relocate_path(&file_name);
            writer.start_file(relocated_file_name, options)?;
            writer.write_all(&buffer)?;
        }

        writer.finish()?;
        Ok(())
    }
}

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write, Seek};
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

    pub fn process<P: AsRef<Path>>(&self, input_path: P, output_path: P) -> Result<(), Box<dyn std::error::Error>> {
        let in_file = File::open(input_path)?;
        let mut archive = ZipArchive::new(in_file)?;

        // Шаг 1: Сканирование точек входа и сбор классов (если включен tree-shaking)
        let mut entry_points = HashSet::new();
        for ep in &self.custom_entry_points {
            entry_points.insert(ep.clone());
        }

        let mut class_map: HashMap<String, ClassFile> = HashMap::new();
        let mut raw_files: HashMap<String, Vec<u8>> = HashMap::new();

        let num_files = archive.len();
        for i in 0..num_files {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();

            if file.is_dir() {
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

        // Шаг 2: Tree-shaking (если активирован)
        let reachable = if self.tree_shake {
            let shaker = TreeShaker::new(entry_points, self.keep_prefixes.clone());
            Some(shaker.compute_reachable(&class_map))
        } else {
            None
        };

        // Шаг 3: Запись выходного JAR
        let out_file = File::create(output_path)?;
        let mut writer = ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut added_dirs = HashSet::new();

        // 3.1. Запись отрелоцированных и отфильтрованных классов
        for (class_name, mut cf) in class_map {
            if let Some(ref reach) = reachable {
                // Если включен tree-shaking и класс не в графе достижимости:
                // Проверяем: если класс принадлежит релоцируемой библиотеке и не достижим -> отсекаем его!
                let is_relocated_target = self.relocator.rules.iter().any(|r| {
                    class_name.starts_with(&r.from_slash)
                });
                if is_relocated_target && !reach.contains(&class_name) {
                    continue; // Пропускаем мертвый класс!
                }
            }

            // Применяем релокацию constant pool
            self.relocator.relocate_class(&mut cf);

            // Релоцируем имя файла
            let original_file_name = format!("{}.class", class_name);
            let relocated_file_name = format!("{}.class", self.relocator.relocate_path(&class_name));

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

        // 3.2. Запись остальных ресурсов и META-INF/services/
        for (file_name, buffer) in raw_files {
            if file_name.starts_with("META-INF/services/") {
                if let Ok(text) = std::str::from_utf8(&buffer) {
                    let (new_file_name, new_content, _) =
                        self.relocator.relocate_service_resource(&file_name, text);
                    writer.start_file(new_file_name, options)?;
                    writer.write_all(new_content.as_bytes())?;
                    continue;
                }
            }

            // Обычный ресурс (например, plugin.yml или картинка)
            let relocated_file_name = self.relocator.relocate_path(&file_name);
            writer.start_file(relocated_file_name, options)?;
            writer.write_all(&buffer)?;
        }

        writer.finish()?;
        Ok(())
    }
}

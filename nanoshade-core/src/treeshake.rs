use std::collections::{HashMap, HashSet, VecDeque};
use crate::classfile::ClassFile;

pub struct TreeShaker {
    pub entry_points: HashSet<String>,
    pub keep_prefixes: Vec<String>,
    pub service_providers: HashSet<String>,
}

impl TreeShaker {
    pub fn new(
        entry_points: HashSet<String>,
        keep_prefixes: Vec<String>,
        service_providers: HashSet<String>,
    ) -> Self {
        Self {
            entry_points,
            keep_prefixes,
            service_providers,
        }
    }

    /// Построить множество достижимых классов на основе графа зависимостей
    pub fn compute_reachable(
        &self,
        classes: &HashMap<String, ClassFile>,
    ) -> (HashSet<String>, HashSet<String>) {
        let mut reachable = HashSet::new();
        let mut project_classes = HashSet::new();
        let mut queue = VecDeque::new();

        // 1. Вычисляем префиксы пакетов самого проекта на основе точек входа
        let mut detected_project_prefixes = self.keep_prefixes.clone();
        for ep in &self.entry_points {
            let slash_ep = ep.replace('.', "/");
            if let Some(slash_idx) = slash_ep.rfind('/') {
                let pkg_prefix = slash_ep[..slash_idx].to_string();
                if !detected_project_prefixes.contains(&pkg_prefix) {
                    detected_project_prefixes.push(pkg_prefix);
                }
            }
        }

        // 2. Все классы проекта автоматически добавляются в корень графа
        for (name, _) in classes {
            let mut is_proj = false;
            for prefix in &detected_project_prefixes {
                let slash_prefix = prefix.replace('.', "/");
                if name.starts_with(&slash_prefix) {
                    is_proj = true;
                    break;
                }
            }
            if is_proj {
                project_classes.insert(name.clone());
                if reachable.insert(name.clone()) {
                    queue.push_back(name.clone());
                }
            }
        }

        // 3. Добавляем явные точки входа (если они ещё не добавлены)
        for ep in &self.entry_points {
            let slash_ep = ep.replace('.', "/");
            if classes.contains_key(&slash_ep) && reachable.insert(slash_ep.clone()) {
                queue.push_back(slash_ep);
            }
        }

        // 4. Добавляем классы сервисов из META-INF/services/
        for sp in &self.service_providers {
            let slash_sp = sp.replace('.', "/");
            if classes.contains_key(&slash_sp) && reachable.insert(slash_sp.clone()) {
                queue.push_back(slash_sp);
            }
        }

        // 5. Обход в ширину (BFS) по всем ссылкам байткода и LDC рефлексии
        while let Some(current_class) = queue.pop_front() {
            if let Some(cf) = classes.get(&current_class) {
                let refs = cf.extract_referenced_classes();
                for r in refs {
                    if classes.contains_key(&r) && reachable.insert(r.clone()) {
                        queue.push_back(r);
                    }
                }
            }
        }

        (reachable, project_classes)
    }

    /// Извлечь Main-Class из MANIFEST.MF
    pub fn extract_manifest_main_class(manifest: &str) -> Option<String> {
        for line in manifest.lines() {
            let trimmed = line.trim();
            if let Some(main) = trimmed.strip_prefix("Main-Class:") {
                return Some(main.trim().replace('.', "/"));
            }
        }
        None
    }

    /// Извлечь main / bootstrapper / loader из plugin.yml или paper-plugin.yml
    pub fn extract_plugin_yml_main(plugin_yml: &str) -> Vec<String> {
        let mut result = Vec::new();
        for line in plugin_yml.lines() {
            let trimmed = line.trim();
            if let Some(main) = trimmed.strip_prefix("main:") {
                let clean = main.trim().trim_matches('\'').trim_matches('"');
                if !clean.is_empty() {
                    result.push(clean.replace('.', "/"));
                }
            } else if let Some(bootstrapper) = trimmed.strip_prefix("bootstrapper:") {
                let clean = bootstrapper.trim().trim_matches('\'').trim_matches('"');
                if !clean.is_empty() {
                    result.push(clean.replace('.', "/"));
                }
            } else if let Some(loader) = trimmed.strip_prefix("loader:") {
                let clean = loader.trim().trim_matches('\'').trim_matches('"');
                if !clean.is_empty() {
                    result.push(clean.replace('.', "/"));
                }
            }
        }
        result
    }

    /// Релоцировать точки входа прямо внутри plugin.yml / paper-plugin.yml
    pub fn relocate_plugin_yml(plugin_yml: &str, relocator: &crate::relocator::Relocator) -> String {
        let mut out_lines = Vec::new();
        for line in plugin_yml.lines() {
            let trimmed = line.trim();
            if let Some(main_val) = trimmed.strip_prefix("main:") {
                let clean = main_val.trim().trim_matches('\'').trim_matches('"');
                let slash = clean.replace('.', "/");
                let relocated = relocator.relocate_path(&slash).replace('/', ".");
                let indent = &line[..line.len() - line.trim_start().len()];
                out_lines.push(format!("{}main: {}", indent, relocated));
            } else if let Some(boot_val) = trimmed.strip_prefix("bootstrapper:") {
                let clean = boot_val.trim().trim_matches('\'').trim_matches('"');
                let slash = clean.replace('.', "/");
                let relocated = relocator.relocate_path(&slash).replace('/', ".");
                let indent = &line[..line.len() - line.trim_start().len()];
                out_lines.push(format!("{}bootstrapper: {}", indent, relocated));
            } else if let Some(loader_val) = trimmed.strip_prefix("loader:") {
                let clean = loader_val.trim().trim_matches('\'').trim_matches('"');
                let slash = clean.replace('.', "/");
                let relocated = relocator.relocate_path(&slash).replace('/', ".");
                let indent = &line[..line.len() - line.trim_start().len()];
                out_lines.push(format!("{}loader: {}", indent, relocated));
            } else {
                out_lines.push(line.to_string());
            }
        }
        out_lines.join("\n")
    }

    /// Извлечь классы провайдеров из META-INF/services/
    pub fn extract_services_classes(content: &str) -> Vec<String> {
        let mut result = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && !trimmed.is_empty() {
                result.push(trimmed.replace('.', "/"));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_manifest_main() {
        let manifest = "Manifest-Version: 1.0\r\nMain-Class: com.nanodev.Main\r\n";
        assert_eq!(
            TreeShaker::extract_manifest_main_class(manifest),
            Some("com/nanodev/Main".to_string())
        );
    }

    #[test]
    fn test_extract_plugin_yml() {
        let plugin_yml = "name: NanoPlugin\nversion: 1.0.1\nmain: com.nanodev.nano.NanoPlugin\nloader: com.nanodev.nano.CustomLoader\n";
        assert_eq!(
            TreeShaker::extract_plugin_yml_main(plugin_yml),
            vec![
                "com/nanodev/nano/NanoPlugin".to_string(),
                "com/nanodev/nano/CustomLoader".to_string()
            ]
        );
    }

    #[test]
    fn test_relocate_plugin_yml() {
        use crate::relocator::{RelocationRule, Relocator};
        let plugin_yml = "name: NanoPlugin\nmain: com.zaxxer.hikari.HikariConfig\nbootstrapper: com.zaxxer.hikari.Boot\n";
        let relocator = Relocator::new(vec![RelocationRule::new("com.zaxxer.hikari", "com.nanodev.libs.hikari")]);
        let relocated = TreeShaker::relocate_plugin_yml(plugin_yml, &relocator);
        assert!(relocated.contains("main: com.nanodev.libs.hikari.HikariConfig"));
        assert!(relocated.contains("bootstrapper: com.nanodev.libs.hikari.Boot"));
    }
}

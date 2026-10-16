use std::collections::{HashMap, HashSet, VecDeque};
use crate::classfile::ClassFile;

pub struct TreeShaker {
    pub entry_points: HashSet<String>,
    pub keep_prefixes: Vec<String>,
}

impl TreeShaker {
    pub fn new(entry_points: HashSet<String>, keep_prefixes: Vec<String>) -> Self {
        Self {
            entry_points,
            keep_prefixes,
        }
    }

    /// Построить множество достижимых классов на основе графа зависимостей
    pub fn compute_reachable(
        &self,
        classes: &HashMap<String, ClassFile>,
    ) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        // 1. Добавляем явные точки входа
        for ep in &self.entry_points {
            let slash_ep = ep.replace('.', "/");
            if classes.contains_key(&slash_ep) && reachable.insert(slash_ep.clone()) {
                queue.push_back(slash_ep);
            }
        }

        // 2. Добавляем классы проекта (по префиксам keep или все, что не попадает под шейкинг)
        for (name, _) in classes {
            for prefix in &self.keep_prefixes {
                let slash_prefix = prefix.replace('.', "/");
                if name.starts_with(&slash_prefix) {
                    if reachable.insert(name.clone()) {
                        queue.push_back(name.clone());
                    }
                    break;
                }
            }
        }

        // 3. Обход в ширину (BFS) по всем ссылкам байткода
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

        reachable
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

    /// Извлечь main / bootstrapper из plugin.yml
    pub fn extract_plugin_yml_main(plugin_yml: &str) -> Vec<String> {
        let mut result = Vec::new();
        for line in plugin_yml.lines() {
            let trimmed = line.trim();
            if let Some(main) = trimmed.strip_prefix("main:") {
                let clean = main.trim().trim_matches('\'').trim_matches('"');
                result.push(clean.replace('.', "/"));
            } else if let Some(bootstrapper) = trimmed.strip_prefix("bootstrapper:") {
                let clean = bootstrapper.trim().trim_matches('\'').trim_matches('"');
                result.push(clean.replace('.', "/"));
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
        let plugin_yml = "name: NanoPlugin\nversion: 1.0.0\nmain: com.nanodev.nano.NanoPlugin\n";
        assert_eq!(
            TreeShaker::extract_plugin_yml_main(plugin_yml),
            vec!["com/nanodev/nano/NanoPlugin".to_string()]
        );
    }
}

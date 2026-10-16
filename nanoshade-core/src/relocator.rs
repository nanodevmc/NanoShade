use crate::classfile::{ClassFile, CpEntry};

#[derive(Debug, Clone)]
pub struct RelocationRule {
    pub from_slash: String,
    pub to_slash: String,
    pub from_dot: String,
    pub to_dot: String,
}

impl RelocationRule {
    pub fn new(from: &str, to: &str) -> Self {
        let from_slash = from.replace('.', "/");
        let to_slash = to.replace('.', "/");
        let from_dot = from.replace('/', ".");
        let to_dot = to.replace('/', ".");
        Self {
            from_slash,
            to_slash,
            from_dot,
            to_dot,
        }
    }
}

pub struct Relocator {
    pub rules: Vec<RelocationRule>,
}

impl Relocator {
    pub fn new(rules: Vec<RelocationRule>) -> Self {
        Self { rules }
    }

    /// Релокация пути внутри JAR
    pub fn relocate_path(&self, path: &str) -> String {
        let mut result = path.to_string();
        for rule in &self.rules {
            if result.starts_with(&rule.from_slash) {
                result = format!("{}{}", rule.to_slash, &result[rule.from_slash.len()..]);
            }
        }
        result
    }

    /// Релокация строки Utf8 (LDC, сигнатуры, дескрипторы полей и методов)
    pub fn relocate_string(&self, input: &str) -> (String, bool) {
        let mut modified = false;
        let mut current = input.to_string();

        for rule in &self.rules {
            if current.contains(&rule.from_slash) {
                current = current.replace(&rule.from_slash, &rule.to_slash);
                modified = true;
            }
            if current.contains(&rule.from_dot) {
                current = current.replace(&rule.from_dot, &rule.to_dot);
                modified = true;
            }
        }

        (current, modified)
    }

    /// Релокация class-файла
    pub fn relocate_class(&self, class: &mut ClassFile) -> bool {
        let mut changed = false;
        for entry in class.constant_pool.iter_mut() {
            if let CpEntry::Utf8(s) = entry {
                let (new_s, was_modified) = self.relocate_string(s);
                if was_modified {
                    *s = new_s;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Релокация ресурсов в META-INF/services/
    pub fn relocate_service_resource(&self, file_name: &str, content: &str) -> (String, String, bool) {
        // Релокация имени интерфейса в META-INF/services/com.example.Foo
        let mut new_file_name = file_name.to_string();
        let mut changed = false;

        if let Some(service_name) = file_name.strip_prefix("META-INF/services/") {
            let (relocated_service, service_mod) = self.relocate_string(service_name);
            if service_mod {
                new_file_name = format!("META-INF/services/{}", relocated_service);
                changed = true;
            }
        }

        // Релокация содержимого файла (классы реализаций)
        let mut new_lines = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                new_lines.push(line.to_string());
            } else {
                let (relocated_impl, impl_mod) = self.relocate_string(trimmed);
                if impl_mod {
                    changed = true;
                }
                new_lines.push(relocated_impl);
            }
        }

        (new_file_name, new_lines.join("\n"), changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relocation_rules() {
        let rule = RelocationRule::new("com.zaxxer.hikari", "com.nanodev.libs.hikari");
        let relocator = Relocator::new(vec![rule]);

        // Slash path
        assert_eq!(
            relocator.relocate_path("com/zaxxer/hikari/HikariConfig.class"),
            "com/nanodev/libs/hikari/HikariConfig.class"
        );

        // Descriptor
        let (res, modified) = relocator.relocate_string("(Lcom/zaxxer/hikari/HikariConfig;)V");
        assert!(modified);
        assert_eq!(res, "(Lcom/nanodev/libs/hikari/HikariConfig;)V");

        // Dotted LDC string
        let (res, modified) = relocator.relocate_string("com.zaxxer.hikari.HikariDataSource");
        assert!(modified);
        assert_eq!(res, "com.nanodev.libs.hikari.HikariDataSource");
    }

    #[test]
    fn test_services_relocation() {
        let rule = RelocationRule::new("org.slf4j", "com.nanodev.libs.slf4j");
        let relocator = Relocator::new(vec![rule]);

        let file_name = "META-INF/services/org.slf4j.spi.SLF4JServiceProvider";
        let content = "# Provider implementation\norg.slf4j.simple.SimpleServiceProvider\n";

        let (new_file, new_content, modified) =
            relocator.relocate_service_resource(file_name, content);

        assert!(modified);
        assert_eq!(new_file, "META-INF/services/com.nanodev.libs.slf4j.spi.SLF4JServiceProvider");
        assert!(new_content.contains("com.nanodev.libs.slf4j.simple.SimpleServiceProvider"));
    }
}

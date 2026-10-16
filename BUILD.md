# Инструкция по сборке NanoShade

## Версии
- Версия проекта: `1.0.0`
- Версия Maven-плагина: `1.0.0`

## Требования
- Rust 1.75+ (в GitHub Actions или локально при наличии cargo)
- Java 8+ / JDK 17+
- Apache Maven 3.8+

## Локальная сборка ядра (Rust)
```cmd
cd nanoshade-core
cargo build --release
```
Бинарник создается в `target/release/nanoshade.exe` (Windows) или `nanoshade` (Linux).

## Локальная сборка Maven Plugin
```cmd
cd nanoshade-maven-plugin
mvn clean install -DskipTests
```

## Автоматическая сборка в CI
GitHub Actions (`.github/workflows/ci.yml`):
- Собирает нативные бинарники под Windows (`x86_64-pc-windows-msvc`) и Linux (`x86_64-unknown-linux-gnu`).
- Прогоняет сквозные интеграционные тесты с релокацией тестового JAR.
- Публикует релизные бинарники в GitHub Releases.
- Публикует Maven-плагин в GitHub Packages и Maven Central.

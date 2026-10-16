# PROJECT_STATE: NanoShade

## 1. Описание
NanoShade — нативная (Rust), потоковая замена maven-shade-plugin и jarjar для молниеносной релокации байткода JVM и tree-shaking на слабых машинах (<128 МБ RAM).

## 2. Структура
- `nanoshade-core/`: Rust CLI-ядро (потоковый парсер/рерайтер class-файлов, zip I/O, tree-shaker).
- `nanoshade-maven-plugin/`: Легковесный Maven Mojo wrapper под Windows/Linux.
- `com.nanodev.NanoShade.yml`: Flatpak-манифест для Flathub/Linux.
- `.github/workflows/ci.yml`: Матричный CI (Windows/Ubuntu), тесты, релизы бинарников и публикация в GitHub Packages & Maven Central.

## 3. Принятые решения
- Ядро пишется на Rust: стриминговый ZIP без распаковки на диск, zero-allocation парсер constant pool JVM.
- Память жестко ограничена: < 128 МБ RAM, работает мгновенно даже на Intel Pentium P4600.
- Репозиторий: `NanoDev1488/NanoShade` (публичный).
- Секреты OSSRH (Sonatype) загружены в GitHub Secrets через API.
- Поддержка точек входа: `META-INF/MANIFEST.MF` (Main-Class), `plugin.yml` (Spigot/Paper) и кастомные флаги.
- Релокация ресурсов: файлы в `META-INF/services/` переименовываются и их содержимое переписывается под новые пакеты.

## 4. Текущий статус
- В процессе: реализация Rust-движка (ClassFile parser/rewriter, ZIP streamer, TreeShaker), Maven-плагина и CI/CD workflow.

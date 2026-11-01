# PROJECT_STATE: NanoShade

## 1. Описание
NanoShade — нативная (Rust), потоковая замена maven-shade-plugin и jarjar для молниеносной релокации байткода JVM и tree-shaking на слабых машинах (<128 МБ RAM).

## 2. Структура
- `nanoshade-core/`: Rust CLI-ядро (потоковый парсер/рерайтер class-файлов, zip I/O, глобальный tree-shaker, JAR cleaner).
- `nanoshade-maven-plugin/`: Легковесный Maven Mojo wrapper под Windows/Linux с UpdateChecker и Flatpak-поддержкой.
- `com.nanodev.NanoShade.yml`: Flatpak-манифест для Flathub/Linux.
- `.github/workflows/ci.yml`: Матричный CI (Windows/Ubuntu), тесты, автогенерация GPG, релизы бинарников и деплой в GitHub Packages и Central Portal.

## 3. Принятые решения
- Версия 1.0.1: автоматическое вырезание неиспользуемых классов ВСЕХ сторонних библиотек.
- Сохранение рефлексии: LDC строки сканируются в Constant Pool живых классов для защиты Class.forName.
- Чистка мусора: автоматическое удаление сигнатур META-INF/*.SF/DSA/RSA во избежание SecurityException.
- Update Checker: асинхронная проверка обновлений с таймаутом (без краша при оффлайне).
- Flatpak: запуск через `flatpak run com.nanodev.NanoShade` при указании `<useFlatpak>true</useFlatpak>`.
- Maven Central Portal: интеграция `central-publishing-maven-plugin` под `central.sonatype.com`.

## 4. Текущий статус (v1.0.1 / v1.0.2 dev)
- Android Termux ARM64: нативная сборка и интеграция подтверждена на реальном устройстве пользователя (EryBuyer v3 собрался за 35.63ms!).
- v1.0.2 (локально): SPI merge, Multi-Release strip, Resource minimizer, smart plugin.yml relocator, инкрементальный кэш.
- GitHub аккаунт пользователя временно заблокирован: коммиты и пуши на паузе до разблокировки.

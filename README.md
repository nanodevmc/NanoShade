# NanoShade ⚡

**Ultra-fast native JVM bytecode relocator & tree shaker** written in Rust.

Разработано специально для мгновенной сборки и релокации Java/Spigot/Paper проектов даже на слабых машинах (Intel Pentium, 4 ГБ ОЗУ) с потреблением памяти `< 128 МБ RAM`. Заменяет медленный `maven-shade-plugin` и `jarjar`.

---

## 🚀 Ключевые возможности
- **Нативная скорость (Rust Core)**: Потоковая правка байткода и Constant Pool без накладных расходов тяжелой JVM.
- **Минимальное потребление памяти**: Потоковый I/O ZIP архива без распаковки на диск (< 64–128 МБ RAM).
- **Тотальный автоматический Tree Shaking**: Автоматически анализирует точки входа (`Main-Class` из `MANIFEST.MF` или `main`/`bootstrapper` из `plugin.yml` для Spigot/Paper) и вычищает мертвые классы **всех сторонних библиотек** без необходимости указывать их вручную!
- **Безопасность Reflection (LDC)**: Парсит строковые константы байткода, защищая классы, загружаемые динамически через `Class.forName(...)`.
- **Очистка от мусора и сжатие**: Удаление конфликтующих подписей безопасности (`META-INF/*.SF`, `*.DSA`, `*.RSA`), лишних Maven-манифестов и максимальное сжатие Deflate.
- **Встроенный Update Checker**: Уведомляет прямо в консоли Maven, если вышла более свежая версия плагина (с таймаутом и защитой от оффлайна).
- **Поддержка Flatpak**: Возможность запускать процесс релокации внутри Flatpak-песочницы через параметр `<useFlatpak>true</useFlatpak>`.
- **Кроссплатформенность**: Готовые нативные бинарники под Windows (`nanoshade.exe`) и Linux (`nanoshade`).

---

## 📦 Подключение в Maven (`pom.xml`)

### 1. Подключение репозитория
Если плагин подключается из GitHub Packages:

```xml
<pluginRepositories>
    <pluginRepository>
        <id>github</id>
        <url>https://maven.pkg.github.com/NanoDev1488/NanoShade</url>
    </pluginRepository>
</pluginRepositories>
```
*(Для Maven Central Portal после синхронизации namespace `io.github.nanodev1488` сторонние репозитории указывать не требуется).*

### 2. Подключение плагина NanoShade

Замените стандартный `maven-shade-plugin` в секции `<build><plugins>`:

```xml
<plugin>
    <groupId>io.github.nanodev1488</groupId>
    <artifactId>nanoshade-maven-plugin</artifactId>
    <version>1.0.1</version>
    <executions>
        <execution>
            <phase>package</phase>
            <goals>
                <goal>shade</goal>
            </goals>
            <configuration>
                <!-- Тотальное вырезание мертвого байткода ВСЕХ библиотек -->
                <treeShake>true</treeShake>

                <!-- Опционально: использовать Flatpak вместо нативного бинарника -->
                <!-- <useFlatpak>true</useFlatpak> -->

                <!-- Правила релокации пакетов -->
                <relocations>
                    <relocation>
                        <pattern>com.zaxxer.hikari</pattern>
                        <shadedPattern>com.nanodev.libs.hikari</shadedPattern>
                    </relocation>
                    <relocation>
                        <pattern>org.slf4j</pattern>
                        <shadedPattern>com.nanodev.libs.slf4j</shadedPattern>
                    </relocation>
                </relocations>
            </configuration>
        </execution>
    </executions>
</plugin>
```

---

## 🐧 Использование через Flatpak

NanoShade можно запускать в изолированной Linux-песочнице Flatpak:

### Локальная сборка и установка манифеста:
```bash
flatpak-builder --user --install build-dir com.nanodev.NanoShade.yml
```

### Запуск релокации в Maven через Flatpak:
В `pom.xml` добавьте:
```xml
<useFlatpak>true</useFlatpak>
```
Либо передайте флаг при сборке проекта:
```bash
mvn clean package -DuseFlatpak=true
```

### Запуск через CLI Flatpak:
```bash
flatpak run com.nanodev.NanoShade --input app.jar --output app-shaded.jar --relocate com.zaxxer.hikari=com.nanodev.libs.hikari --tree-shake
```

---

## 💻 Использование через нативный CLI

Готовые бинарники доступны в [GitHub Releases](https://github.com/NanoDev1488/NanoShade/releases/latest).

```cmd
nanoshade --input app.jar --output app-shaded.jar --relocate com.zaxxer.hikari=com.nanodev.libs.hikari --tree-shake
```

### Параметры CLI:
- `-i, --input <PATH>`: Входной JAR архив.
- `-o, --output <PATH>`: Выходной оптимизированный JAR.
- `-r, --relocate <FROM=TO>`: Правило релокации (`from=to` или `from->to`, можно указывать несколько раз).
- `--tree-shake`: Включить автоматическую очистку неиспользуемых классов всех библиотек.
- `--entry-point <CLASS>`: Дополнительная точка входа для графа зависимостей.
- `--keep <PREFIX>`: Префикс пакета, который никогда не удаляется шейкером.

---

## 🛠 Сборка из исходников

### Rust Core:
```cmd
cd nanoshade-core
cargo build --release
```
Бинарник появится в `target/release/nanoshade.exe` (Windows) или `nanoshade` (Linux).

### Maven Plugin:
```cmd
cd nanoshade-maven-plugin
mvn clean install -DskipTests
```

---

## 👤 Автор & Контакты
- **Разработчик**: NanoDev
- **Telegram канал**: [t.me/WineSquad](https://t.me/WineSquad)
- **Связь**: [t.me/radoqi](https://t.me/radoqi)
- **Лицензия**: MIT

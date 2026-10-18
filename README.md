# NanoShade ⚡

**Ultra-fast native JVM bytecode relocator & tree shaker** written in Rust.

Разработано специально для мгновенной сборки и релокации Java/Spigot/Paper проектов даже на слабых машинах (Intel Pentium, 4 ГБ ОЗУ) с потреблением памяти `< 128 МБ RAM`. Заменяет медленный `maven-shade-plugin` и `jarjar`.

---

## 🚀 Преимущества
- **Нативная скорость (Rust)**: Потоковая правка байткода и Constant Pool без overhead'а тяжелой JVM.
- **Минимальное потребление памяти**: Потоковый I/O ZIP архива без распаковки на диск (<64–128 МБ RAM).
- **Тотальный автоматический Tree Shaking**: Автоматически анализирует точки входа (`Main-Class` из `MANIFEST.MF` или `main`/`bootstrapper` из `plugin.yml`) и вычищает мертвые классы **всех сторонних библиотек** без необходимости указывать их вручную, с полной защитой Reflection (LDC) и `META-INF/services/`.
- **Очистка от мусора и сжатие**: Удаление конфликтующих подписей безопасности (`META-INF/*.SF`, `*.DSA`, `*.RSA`), лишних Maven-манифестов и оптимизация размера сжатия.
- **Встроенный Update Checker**: Уведомляет в логах сборки Maven, если вышла более свежая версия плагина (с таймаутом и защитой от сбоев сети).
- **Поддержка Flatpak**: Возможность запускать процесс релокации внутри Flatpak-песочницы через `<useFlatpak>true</useFlatpak>`.
- **Готовый Maven Plugin**: Работает «из коробки» на Windows и Linux.

---

## 📦 Подключение в Maven (`pom.xml`)

Замените стандартный `maven-shade-plugin`:

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
                <!-- Тотальное вырезание мертвого байткода всех библиотек -->
                <treeShake>true</treeShake>
                <!-- Опционально: запускать через Flatpak песочницу -->
                <!-- <useFlatpak>true</useFlatpak> -->
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

## 💻 Использование через CLI

```cmd
nanoshade --input app.jar --output app-shaded.jar --relocate com.zaxxer.hikari=com.nanodev.libs.hikari --tree-shake
```

### Параметры CLI:
- `-i, --input <PATH>`: Входной JAR архив.
- `-o, --output <PATH>`: Выходной оптимизированный JAR.
- `-r, --relocate <FROM=TO>`: Правило релокации (можно указывать несколько раз).
- `--tree-shake`: Включить очистку неиспользуемых классов всех библиотек.
- `--entry-point <CLASS>`: Дополнительная точка входа для графа зависимостей.
- `--keep <PREFIX>`: Префикс пакета, который никогда не удаляется шейкером.

---

## 🛠 Сборка из исходников

### Rust Core:
```cmd
cd nanoshade-core
cargo build --release
```

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

import sys
import os

target_pom = os.path.join("test-nanomarket", "pom.xml")
if not os.path.exists(target_pom):
    print(f"Error: {target_pom} not found")
    sys.exit(1)

with open(target_pom, "r", encoding="utf-8") as f:
    content = f.read()

plugin_xml = """
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
                            <treeShake>true</treeShake>
                            <relocations>
                                <relocation>
                                    <pattern>io.nano.market</pattern>
                                    <shadedPattern>io.nano.market.shaded</shadedPattern>
                                </relocation>
                            </relocations>
                        </configuration>
                    </execution>
                </executions>
            </plugin>
"""

idx = content.find("</plugins>")
if idx == -1:
    print("Could not find </plugins> in pom.xml")
    sys.exit(1)

new_content = content[:idx] + plugin_xml + content[idx:]
with open(target_pom, "w", encoding="utf-8") as f:
    f.write(new_content)

print("Successfully injected NanoShade plugin into NanoMarket pom.xml")

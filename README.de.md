# KOKURA

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | **Deutsch**
<!-- readme-language-links:end -->

**[KOKURA · HTML-Handbuch](https://bartaro.github.io/kitaq-docs/de/kokura.html)**

GB/GBC-Emulator mit Kommandozeile, Ablaufprotokollen, Debugger und Python-/C-Schnittstellen.

Öffentliche Vorabversion: APIs und Verhalten können sich noch ändern.

## Fertige Windows-Version

Im Stammverzeichnis des Repositorys liegt `kokura-cli.exe`, als Release für Windows x64 kompiliert. Das Programm wird über die Kommandozeile bedient. Zum Ausführen benötigen Sie weder Rust noch Python oder .NET. Laden Sie das Repository als ZIP herunter, damit die ausführbare Datei und die Lizenzhinweise zusammenbleiben. Die native C-Laufzeitbibliothek ist statisch eingebunden; das Programm verwendet System-DLLs von Windows.

```powershell
.\kokura-cli.exe --help
```

Mit `.\scripts\build.ps1` bauen Sie ausschließlich dieses Kommandozeilenprogramm neu; bei Bedarf ergänzen Sie `-Offline`. Das Skript kopiert die ausführbare Datei ins Stammverzeichnis. Grafische Oberflächen, Python-Erweiterungsmodule und DLLs der C-API gehören nicht zu diesem Paket ausführbarer Programme. Einzelheiten finden Sie im [Build-Nachweis](BINARY_BUILD.json) und in den [Lizenzhinweisen zu den Binärabhängigkeiten](BINARY_NOTICES.md).

Lizenzgeber des eigenständigen Projektcodes ist **DAISUKE OBA**. Dieser Code steht unter der MIT-Lizenz. Geben Sie bei einer Weiterverteilung `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` und das Verzeichnis `licenses/` mit. Für Bibliotheken Dritter gelten deren jeweilige Rechteinhaber und Lizenzbedingungen.

## Selbst kompilieren und starten

Verwenden Sie eine aktuelle stabile Rust-Toolchain. Installieren Sie unter Windows außerdem Visual Studio Build Tools mit dem Workload für die Desktopentwicklung mit C++.

```powershell
.\scripts\build.ps1
.\kokura-cli.exe --help
```

## Handbücher und Lizenzen

- [KOKURA · HTML-Handbuch](https://bartaro.github.io/kitaq-docs/de/kokura.html)
- [Englisches Handbuch](https://bartaro.github.io/kitaq-docs/en/kokura.html) / [Japanisches Handbuch](https://bartaro.github.io/kitaq-docs/kokura.html)
- [Handbuchquellen zum Offline-Lesen](https://github.com/bartaro/kitaq-docs)
- [Lizenz](LICENSE) / [Japanische Übersetzung zur Orientierung](LICENSE.ja)

Die Projektlizenz ersetzt keine Bedingungen Dritter für Abhängigkeiten, Logos oder Marken. Behalten Sie bei einer Weitergabe die beigefügten Hinweise bei.


<!-- native-platform-binaries-20261004-de -->
### Vorgefertigte CLI für Linux und macOS

Die in GitHub Actions durch Ausführung geprüften CLI-Dateien befinden sich in den folgenden Ordnern. Für ihre Ausführung sind weder Rust noch Python noch .NET erforderlich. Die Linux-Version ist für x86_64/glibc vorgesehen; wählen Sie unter macOS die passende CPU-Version.

| OS / CPU | CLI |
| --- | --- |
| Linux x86_64 (glibc) | [bin/linux-x86_64/kokura-cli](bin/linux-x86_64/kokura-cli) |
| macOS ARM64 | [bin/macos-arm64/kokura-cli](bin/macos-arm64/kokura-cli) |
| macOS Intel | [bin/macos-x86_64/kokura-cli](bin/macos-x86_64/kokura-cli) |

```sh
chmod +x bin/linux-x86_64/kokura-cli
./bin/linux-x86_64/kokura-cli --help

chmod +x bin/macos-arm64/kokura-cli
./bin/macos-arm64/kokura-cli --help

chmod +x bin/macos-x86_64/kokura-cli
./bin/macos-x86_64/kokura-cli --help
```

Führen Sie die Befehle im Stammverzeichnis des Repositorys aus. Bewahren Sie bei der Weitergabe LICENSE, LICENSE.ja, BINARY_NOTICES.md und licenses/ auf. NATIVE_BINARIES.json enthält Prüfsummen, Abhängigkeiten, Quellcoderevisionen und die Ergebnisse der nativen Ausführungsprüfungen.

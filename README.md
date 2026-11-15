# KOKURA

**English** | [日本語](README.md#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

<a name="english"></a>

## English

GB/GBC emulator with CLI, tracing, debugging and Python/C interfaces.

Public preview: APIs and behavior may change.

The 2026-11-14 source update improves DIV/APU timing, CH3 fetches, live register writes and PCM integration. See [APU accuracy notes](APU_ACCURACY.md).

### Prebuilt Windows CLI

The repository root includes `kokura-cli.exe`, built for Windows x64 in Release mode.
It is a command-line program. Rust, Python and .NET are not required to run it.
Download the repository ZIP to keep the executable and license notices together.
The native C runtime is linked statically; the program uses Windows system DLLs.

```powershell
.\kokura-cli.exe --help
```

Rebuild only this CLI with `.\scripts\build.ps1` (optionally `-Offline`).
The script copies the executable to the repository root. GUI frontends, Python
extension modules and C API DLLs are not part of this executable distribution.
See [binary dependency notices](BINARY_NOTICES.md).

The licensor of the independent project code is **DAISUKE OBA**, under MIT.
When redistributing, include `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` and the `licenses/` directory.
Third-party libraries retain their own copyright holders and terms.

### Prebuilt Linux and macOS CLI

Rust, Python and .NET are not required to run the CLI. The Linux build targets x86_64/glibc; choose the matching CPU for macOS.

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

Run the commands from the repository root. For redistribution, retain LICENSE, LICENSE.ja, BINARY_NOTICES.md and licenses/.

### First use

```powershell
.\kokura-cli.exe game.gb --run-frames 120 --png screen.png
```


### Build and first use

A current stable Rust toolchain. On Windows, install Visual Studio Build Tools with the Desktop development with C++ workload.

```powershell
.\scripts\build.ps1
.\kokura-cli.exe --help
```

### Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/kokura.html) / [English manuals](https://bartaro.github.io/kitaq-docs/en/kokura.html)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [Japanese reference translation](LICENSE.ja)

The project license does not replace third-party dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.

---

<a name="japanese"></a>

## 日本語

CLI、トレース、デバッグ、Python・Cインターフェースを備えたGB/GBCエミュレータです。

パブリックプレビュー版です。APIや動作は変更される場合があります。

2026-10-29更新では、パルス波の再発音、スイープ、音量ステップ、レジスタの読み書き、
DMG/CGBの出力フィルタを修正しました。[音源精度の変更点と検証範囲](APU_ACCURACY.md)を参照してください。

### ビルド済みWindows CLI

リポジトリ直下に、Windows x64向けにReleaseモードでビルドした `kokura-cli.exe` を同梱しています。コマンドラインで使うプログラムで、実行時にRust・Python・.NETは不要です。リポジトリのZIPを取得して、実行ファイルと権利表記をまとめて入手してください。ネイティブCランタイムは静的リンクされており、WindowsのシステムDLLを使用します。

```powershell
.\kokura-cli.exe --help
```

このCLIだけを再ビルドするには `.\scripts\build.ps1` を実行します。必要に応じて `-Offline` を指定できます。スクリプトは実行ファイルをリポジトリ直下へコピーします。GUI、Python拡張モジュール、C APIのDLLは、この実行ファイルの配布には含めていません。

[バイナリの依存物に関する権利表記](BINARY_NOTICES.md)を参照してください。配布時は `LICENSE`、`LICENSE.ja`、`BINARY_NOTICES.md` と `licenses/` フォルダーも同梱してください。

独自コードの著作権者・ライセンサーは **DAISUKE OBA** で、MITライセンスを適用しています。第三者のライブラリは、それぞれの著作権者と利用条件を保持します。

### ビルド済みLinux・macOS CLI

CLIの実行にRust・Python・.NETは不要です。Linux版はx86_64/glibc向けです。macOSではCPUに合う版を選んでください。

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

コマンドはリポジトリ直下で実行します。再配布時はLICENSE、LICENSE.ja、BINARY_NOTICES.md、licenses/も保持してください。

### 最初の使い方

```powershell
.\kokura-cli.exe game.gb --run-frames 120 --png screen.png
```


### ビルドと初回利用

再ビルドには現在の安定版Rustツールチェーンが必要です。Windowsでは、Visual Studio Build Toolsの「C++によるデスクトップ開発」ワークロードをインストールしてください。

```powershell
.\scripts\build.ps1
.\kokura-cli.exe --help
```

### マニュアルとライセンス

- [日本語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/kokura.html) / [英語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/en/kokura.html)
- [オフライン用マニュアルのソース](https://github.com/bartaro/kitaq-docs)
- [ライセンス英語原文](LICENSE) / [日本語参考訳](LICENSE.ja)

プロジェクトのライセンスは、第三者の依存ライブラリ、ロゴ、商標に関する条件を置き換えるものではありません。再配布時は付属の権利表記も保持してください。

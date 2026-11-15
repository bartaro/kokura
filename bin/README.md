# KOKURA — Downloads / ダウンロード

Choose the executable for your OS and CPU. Keep the license notices with it.
OSとCPUに合う実行ファイルを選び、ライセンス表記も一緒に保存してください。

| OS / CPU | CLI |
|---|---|
| Windows x64 | [kokura-cli](../kokura-cli.exe) |
| Linux x86_64 (glibc) | [kokura-cli](linux-x86_64/kokura-cli) |
| macOS Apple Silicon | [kokura-cli](macos-arm64/kokura-cli) |
| macOS Intel | [kokura-cli](macos-x86_64/kokura-cli) |

Linux: x86_64/glibc, GLIBC_2.34 symbol requirements.
macOS deployment targets: ARM64 11.0, Intel 10.12.

## Start / 起動

Run from the repository root. / リポジトリのルートで実行します。

```powershell
.\kokura-cli.exe --help
```

```sh
chmod +x bin/linux-x86_64/kokura-cli
./bin/linux-x86_64/kokura-cli --help
```

On macOS, use the matching macOS folder in the same commands.
macOSでは対応するmacOS用フォルダーへ読み替えます。

[Download checksums / ダウンロードのチェックサム](SHA256SUMS)

[Manual / マニュアル](../README.md) · [License](../LICENSE) · [Binary notices](../BINARY_NOTICES.md)

Keep LICENSE, LICENSE.ja, BINARY_NOTICES.md and licenses/ when redistributing.
再配布時は LICENSE、LICENSE.ja、BINARY_NOTICES.md、licenses/ を同梱してください。

# Native binary update — 2026-11-15

[Download the four platforms](bin/README.md) · [SHA-256](bin/SHA256SUMS)

Built from public commit `567d9515cec8dd718655e003253d4db5cfcc30f4` by
[GitHub Actions](https://github.com/bartaro/kokura/actions/runs/43612265492).
Windows x86_64, Linux x86_64, macOS ARM64 and Intel each passed 304 release
tests and native DMG/CGB PCM recording plus save-state/resume checks.
The source includes the 2026-11-14 APU update. Binary bytes and tracked Rust
source hashes were checked against the successful CI artifacts before installation.

Windows statically links the C runtime and imports only system DLLs. Linux
requires x86_64/glibc with GLIBC_2.34 symbols. macOS deployment targets are
11.0 (ARM64) and 10.12 (Intel); execution was checked on the newer CI hosts,
not every older supported OS version. The dependency lockfile is unchanged;
the existing license notices remain included.

See [Windows build details](BINARY_BUILD.json), [Linux/macOS build details](NATIVE_BINARIES.json)
and [all-platform verification](BINARY_UPDATE_20261115.json).
The public [native workflow](.github/workflows/native-cli.yml) reproduces the builds.
No GUI frontend is included. These are automated software checks; physical
consoles and human listening were not tested.

4環境の配布バイナリを更新しました。各環境304件のテストと、DMG/CGBの音声出力・
ステート保存／再開を自動検証しています。上記の配布先とSHA-256を参照してください。

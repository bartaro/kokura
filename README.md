# KOKURA

GB/GBC emulator with CLI, tracing, debugging and Python/C interfaces.

Public preview: APIs and behavior may change.

## Build and first use

A current stable Rust toolchain. On Windows, install Visual Studio Build Tools with the Desktop development with C++ workload.

```powershell
cargo build --release --locked
cargo run --release -p kokura-cli -- --help
```

## Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [日本語参考訳](LICENSE.ja)

The project license does not replace third-party font, dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.

## Public package scope

This publication contains the emulator core, CLI and integration APIs. GUI frontends and the PLITA dependency are deferred and are not included in this repository snapshot.

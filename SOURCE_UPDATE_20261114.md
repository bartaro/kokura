# Source update — 2026-11-14

Compared the current local KOKURA CLI dependency sources with GitHub `main`
(`8f6235b`). Integrated the APU interval mixer, DIV/frame-sequencer and CH3
timing, live envelope/length/sweep behavior and CLI WAV regression tests.
Existing public documentation comments, portable manifests and curated map
test fixture names are retained. The public CLI dependency code otherwise
matches the local implementation; emulator GUI projects are outside this update.

## Validation of this public source tree

`cargo test --offline --locked -p kokura-core -p kokura-debug -p kokura-bridge -p kokura-cli -p kokura-capi --lib --bins -- --quiet`

304 tests passed: core 183, debug 50, bridge 26, CLI 42, C API 3. One existing
manual debug probe is ignored. `cargo build --offline --locked -p kokura-cli
-p kokura-capi` also passed on Windows x86_64 with Rust 1.93.1.

See [the audio guide](APU_ACCURACY.md) for supported behavior, save-state
compatibility and remaining hardware-model limits. This verification is
automated software evidence; physical hardware and human listening were not tested.

## 日本語

GitHub最新版とローカルのCLI依存ソースを比較し、音源精度とWAV回帰テストを更新しました。
公開用の注釈・依存関係・サンプルの整理を維持し、公開コピーで304件のテストとCLI/C APIの
ビルドを確認しました。今回の更新はソースと説明書です。同梱の配布実行ファイルは更新しておらず、
その版・ハッシュは既存のビルド記録に従います。最新音源を使う場合は再ビルドしてください。


[Source hashes and test counts](SOURCE_UPDATE_20261114.json).

# APU accuracy update: 2026-11-14

The CLI, C API and Python interface share the updated Rust core. GBSEQ vendors
the same APU implementation for playback, keyboard monitoring and WAV export.
Native PCM remains 65,536 Hz stereo; no new external dependency is required.

- Pulse, wave and noise outputs are integrated over their timer intervals.
  Long-period noise counters retain their full range.
- DIV falling edges clock the APU sequencer, including DIV writes and CPU
  speed switching. Timer and APU events are ordered within CPU bus execution.
- CH3 models fetch/latch and retrigger behavior, DMG RAM access windows and
  retrigger corruption, with separate CGB access behavior.
- Length enable/trigger edges apply the extra phase-dependent clock. Live
  envelope writes retain countdown state; supported CGB live-volume rules,
  stopped envelopes and trigger-latched sweep enable are covered by tests.
- CLI WAV tests cover restored queued audio, inclusive frame ranges across
  input segments, and powered-off silence with the correct duration.

Save-state version remains 7. Fields reused for APU control latches carry a
marker; older unmarked states infer missing history once. They cannot recover
exact trigger history that was never serialized. A fresh trigger establishes
the new latches. This does not claim complete hardware-revision accuracy.

## 日本語

KOKURA-CLIの共通音源を更新しました。DIVとAPUの同期、CH3の波形読み出し・再トリガー、
長さカウンター、演奏中のエンベロープ・スイープ書き換えを改善し、音声区間の積分から
65,536 HzのステレオPCMを生成します。GBSEQも同じAPUを内蔵します。
ステート形式はバージョン7を維持しますが、古いステートに保存されていなかった
過去のトリガー状態は推定となります。実機の全リビジョンへの完全一致を意味しません。

## Reproduce the checks

```sh
cargo test --locked -p kokura-core -p kokura-debug -p kokura-bridge -p kokura-cli -p kokura-capi --lib --bins
cargo build --locked --release -p kokura-cli
```

See [source verification](SOURCE_UPDATE_20261114.md) and the implementation records:

- [Sample integration and noise range](docs/audio_accuracy_validation_20261113.md)
- [DIV and CH3 timing](docs/audio_timing_validation_20261114.md)
- [Live audio-register writes](docs/audio_register_writes_validation_20261114.md)

The dated implementation records describe earlier local tests, including local
recording fixtures; the source verification records checks of this public tree.
These automated checks do not establish physical-console or listening validation.
[Native executables updated 2026-11-15](bin/README.md) include this source update
on Windows x86_64, Linux x86_64 and macOS ARM64/Intel. `BINARY_BUILD.json`
and `NATIVE_BINARIES.json` record the exact source revision, hashes and native checks.

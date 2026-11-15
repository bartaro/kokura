# Audio behavior and save-state compatibility

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
  stopped envelopes and trigger-latched sweep enable are modeled.
- CLI WAV export includes restored queued audio, frame ranges across input
  segments, and powered-off silence with the requested duration.

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

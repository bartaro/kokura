# Live audio-register refinement (2026-11-14)

## Scope and implementation

This is stage 3 after the DIV/APU and CH3 changes in
[the previous validation record](audio_timing_validation_20261114.md).
The shared Rust core is used by the CLI. Native PCM remains 65,536 Hz, stereo,
signed 16-bit. No GUI, resampler, host queue or frame-pacing changes were made.

- NRx4 length-enable edges apply the extra length clock when the next sequencer
  step does not clock length. Expiry disables the channel unless that write
  also triggers it. Reloading zero length on this phase uses 63 or 255 when
  length is enabled. A nonzero length is not reloaded by a trigger.
- Length counts down even while the channel is inactive. Triggering with the
  DAC off still reloads counters without enabling audible output.
- Pulse retriggers retain the two low bits of the period timer, as well as
  the duty phase already preserved by the earlier update.
- Live envelope writes do not reload initial volume or restart the countdown.
  DAC-off writes still disable immediately. Zero pace counts as eight without
  automatic volume updates. Triggering immediately before the envelope step
  adds one to the countdown. An exhausted envelope stays stopped until trigger.
- CGB live-volume writes use the documented CGB-02/04 zombie transformations.
  The length rule corresponds to DMG and CGB-04/05, not the CGB-02 anomaly.
  DMG uses the portable zero-pace/increasing manual-volume case only; other
  uncertain revision-specific writes retain the current volume.
- CH1 sweep enable is latched at trigger. Writing NR10 cannot enable a sweep
  that was disabled at trigger, and does not restart the current countdown.
  Zero pace suppresses pitch updates without clearing the trigger latch.
  Existing shadow, overflow, second-overflow and negate-clear behavior remains.

## State compatibility

No serialized fields were added and state version remains 7. For CH1/CH2/CH4,
the previously unused wave-sample byte stores marked envelope-running and
sweep-enabled bits. CH3 continues to use its own sample byte normally.

An unmarked legacy state infers these latches once from saved activity and
parameters, before changing NR10/NRx2. Old files did not contain trigger history,
so exact historical latch recovery is impossible after earlier live writes.
Fresh triggers establish exact new latches. New-state roundtrips preserve them.

## Automated checks

Command:

```powershell
cargo test --offline -p kokura-core -p kokura-debug -p kokura-cli -p kokura-capi --lib --bins -- --quiet
```

- Core: 183 passed, including 19 new tests.
- CLI: 42 passed; debug: 50 passed and one manual probe ignored; C API: 3 passed.
- Total: 278 passed, zero failures, one ignored.
- The first 13 new cases produced 11 failures against the pre-change core.
- Matrices cover all four channels, eight sequencer phases, DMG/CGB, old/new
  length enable, zero reload, expiry and retrigger, and inactive-channel clocks.
- Envelope checks cover all three envelope channels, all paces, timer retention,
  CGB live-write rules, endpoint halt, DAC disable and retrigger recovery.
- Sweep tests cover trigger-latched disable, pace changes and zero-pace pause.
- State roundtrip, unmarked legacy inference, reset and traced/fast PCM equality
  are covered. CPU-bus integration crosses the length phase boundary at both
  CPU speeds and in both execution paths.
- An older expiry-observation fixture now explicitly power-cycles NR52 before
  arming length 1. Its old post-boot phase would legitimately expire and reload
  length to 63 under the newly modeled trigger behavior.

## CLI recordings and throughput

Optimized offline builds and targeted formatting checks passed. Fresh-ROM runs:

```powershell
.\target\release\kokura-cli.exe ..\tetris\tetris_512k.gb --hardware cgb --run-frames 600 --input-seq 'NONE:180;START:8;NONE:412' --record-wav verification/audio_register_writes_20261114/tetris.wav --dump-report verification/audio_register_writes_20261114/tetris.json --report-minimal summary
.\target\release\kokura-cli.exe ..\z_selection\saga2.gb --hardware dmg --run-frames 900 --input-seq 'NONE:180;A:8;NONE:712' --record-wav verification/audio_register_writes_20261114/saga2.wav --dump-report verification/audio_register_writes_20261114/saga2.json --report-minimal summary
```

| Run | Mode | Frames run | PCM frames | Duration (s) | Peak |
| --- | --- | ---: | ---: | ---: | ---: |
| Tetris with START | CGB | 600 | 660795 | 10.082932 | 27344 |
| Saga2 with A | DMG | 900 | 987525 | 15.068436 | 18954 |
| Old Tetris state at frame 200, resumed to 600 | CGB | 400 | 439160 | 6.701050 | 27344 |

All recordings are non-silent, stereo 65,536 Hz/16-bit, with zero drops and zero
clipped samples. Their sample counts equal the runtime reports. Fresh Tetris
and Saga2 runs retain the previous build's end-cycle counts (42,290,936 and
63,201,604); the resumed Tetris run also ends at cycle 42,290,936.

The old version 7 state was made by CLI SHA-256
`3DEE4B0DDAAF5368DE0045EB48953D801460E293009005CA660ABDF3484E9BCC`.
State SHA-256:
`B4F362EB65129C1385D1C21E6033FD105D7B6444632B33D33A47EE083CEBC930`.
It was resumed with `--load-state .../legacy_v7.state --run-frames 400`.

Core-only `perf_probe` with audio draining, same input sequences:

| ROM | Before (frames/s) | After (frames/s) |
| --- | ---: | ---: |
| Tetris | 245.07 | 251.66 |
| Saga2 | 163.92 | 166.38 |

These are single-run throughput measurements, not a claimed speedup or GUI FPS
guarantee. No slowdown was observed in this check; both runs reported zero
dropped audio frames. Host-load variability remains.

## Hardware basis and limits

Independent implementation from hardware documentation, not emulator source:

- [Pan Docs audio details](https://github.com/gbdev/pandocs/blob/master/src/Audio_details.md)
- [Pan Docs audio registers](https://github.com/gbdev/pandocs/blob/master/src/Audio_Registers.md)

These checks establish regression behavior against the documented model, not
hardware sound parity. No physical-hardware waveform comparison, subjective
listening comparison or external APU conformance ROM suite was run for this
stage. Silicon-revision-specific DMG zombie behavior, CGB-02 length behavior,
analog calibration and band-limited reconstruction remain outside this change.

## Delivery

- Latest CLI: `dist/kokura-cli.exe`.
- SHA-256: `15C8FCE52F499FA80BA3CFD2170F716B5E68BA1CA3B764DA225B232A5189A126`.
- Previous CLI: `dist/kokura_cli_20261114_timing/kokura-cli.exe`.
- Compact evidence and pre-change source snapshots:
  `verification/audio_register_writes_20261114/`.
- Generated WAVs and the ROM-containing temporary state were removed after
  checking their metrics and hashes. The validator script and reports remain.
- GUI executables were not rebuilt in this CLI task.

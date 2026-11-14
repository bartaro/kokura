# CLI audio accuracy validation (2026-11-13)

## Changes

- Integrate pulse, wave and noise levels over their exact timer intervals before
  mixing. PCM no longer depends on CPU instruction/tick partition sizes.
- Preserve fractional mixer averages until PCM conversion.
- Keep the full CH4 divider countdown, including the 917,504-cycle NR43=0xD7
  case. The previously unused CH4 shadow word stores the upper countdown bits,
  retaining the serialized state layout.
- Preserve the PCM clock and pending audio across NR52 power-off, producing
  silence for powered-off time instead of shortening recordings.
- Discard pre-resume queued PCM at the CLI capture origin without resetting
  the restored APU phase.

## Automated results

- Core: 146 passed. Includes 38 APU tests and nine new accuracy regressions.
- CLI: 42 passed. Includes three new capture-origin/range/silence regressions.
- Debug: 50 passed; one manual profiling probe ignored.
- `rustfmt --check` passed for all three changed Rust files.
- Offline optimized CLI build passed.

The APU cases exercise all 256 NR43 settings, both LFSR widths against the
documented XNOR sequence, width lockup/retrigger, long-divider serialization,
exact one-cycle versus batched integration, register changes, stereo routing
and power-off timing. CLI cases cover normal and observation execution paths.

## Real-ROM WAV smoke

Both recordings used native 65,536 Hz, stereo, signed 16-bit PCM. Sample counts
matched both the report's generated PCM count and floor(elapsed T-cycles / 64).

| ROM | Mode | Frames | T-cycles | PCM frames | Duration (s) | Peak | Dropped | Clipped |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| tetris/tetris_512k.gb | CGB | 600 | 42290936 | 660795 | 10.082932 | 27399 | 0 | 0 |
| z_selection/saga2.gb | DMG | 900 | 63201604 | 987525 | 15.068436 | 19200 | 0 | 0 |

Input sequences were `NONE:180;START:8;NONE:412` and
`NONE:180;A:8;NONE:712`, respectively. Neither run hit an unsupported opcode.
Temporary WAV/report files were removed after analysis.

## Delivery and limits

- Executable: `dist/kokura-cli.exe`.
- SHA-256: `BB5722C6495C6D99800DE0194A8AEBD9DDA0FA5B7C3D688D1FF0F00640E0FAD4`.
- Reference: [Pan Docs audio registers](https://github.com/gbdev/pandocs/blob/master/src/Audio_Registers.md)
  and [audio details](https://github.com/gbdev/pandocs/blob/master/src/Audio_details.md).
- These are numerical/model and offline recording checks, not listening,
  realtime GUI playback or hardware waveform equivalence tests.
- DIV/APU coupling, model-specific envelope and wave-RAM access quirks,
  and band-limited reconstruction remain separate accuracy work.

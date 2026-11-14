# DIV/APU and CH3 refinement (2026-11-14)

## Implementation

The CLI uses the shared `kokura-core`; no separate CLI synthesizer was added.
The native PCM rate remains 65,536 Hz, stereo, signed 16-bit.

- `Machine` synchronizes the APU divider phase with the timer's internal DIV
  counter. Bit 12 is selected at normal CPU speed, bit 13 at double speed.
- Writing DIV clocks the sequencer only if the selected bit was high. CGB
  speed switching resets DIV using the old speed's bit before selecting the
  new speed. The existing speed-switch clock freeze is retained.
- The DIV phase continues while NR52 is off; power-on's first sequencer edge
  clocks step 0. Sound length, envelope and sweep share this edge source.
- CH3 exposes its fetched wave-RAM byte during a two-T-cycle DMG access window.
  Outside the window reads return FF and writes are ignored. CGB active-channel
  accesses alias the current byte; inactive-channel accesses use the address.
- Triggering CH3 preserves the playing sample nibble, waits period + 6 T-cycles
  for its first fetch, and starts at sample index 1. Live period changes leave
  the current countdown intact. CPU RAM writes do not rewrite the sample latch.
- DMG retrigger collisions copy the affected byte/aligned group into wave RAM;
  CGB and non-collision phases leave RAM intact.
- Normal LD/LDH, indirect-register loads/stores, INC/DEC and CB audio accesses
  advance to their bus M-cycle before accessing DIV/APU/PCM/wave RAM. The
  remainder advances once; frame completion and trace events survive splitting.

No state-file fields were added. Two previously unused CH3 fields hold the
fetch-window countdown and marked nibble selector. Legacy selectors fall back
to the saved wave position. Instruction clock progress is transient and is
cleared on ROM/state loading.

## Verification

`cargo test --offline -p kokura-core -p kokura-debug -p kokura-cli -p kokura-capi --lib --bins`

- Core: 164 passed, including 18 added regressions.
- CLI: 42 passed; debug: 50 passed and one manual probe ignored; C API: 3 passed.
- Total: 259 passed, zero failures, one ignored.
- The first six new APU cases failed against the previous implementation and
  passed after the changes.
- Tests cover both CPU speeds, DIV high/low reset, NR52 phase continuity,
  speed switching, CPU bus access timing, wave-window boundaries, all aliased
  addresses, DMG corruption/non-corruption, legacy latch fallback, serialized
  continuation, instruction frame completion and traced/fast PCM equality.
- Formatting checks and the offline optimized CLI build passed.

Real-ROM recordings had native-rate stereo headers, nonzero gameplay audio,
zero dropped samples and zero clipped samples. WAV frame counts matched the
CLI's generated PCM count.

| Run | Mode | Emulated frames | PCM frames | Duration (s) | Peak |
| --- | --- | ---: | ---: | ---: | ---: |
| Tetris, NONE:180;START:8;NONE:412 | CGB | 600 | 660795 | 10.082932 | 27344 |
| Saga2, NONE:180;A:8;NONE:712 | DMG | 900 | 987525 | 15.068436 | 18957 |
| Previous-CLI version 7 state, resume | CGB | 60 additional | 66984 | 1.022095 | 0 |

The legacy-state test resumes the early, silent Tetris title sequence. Its
zero peak is expected and is not an active-music test. The state was generated
by the previous CLI (SHA-256 BB5722C6495C6D99800DE0194A8AEBD9DDA0FA5B7C3D688D1FF0F00640E0FAD4).
The state SHA-256 was 14AEB672C3552A9317FEFE720C01BD2B9973E89F03B3CDF9F53131C06EC5B334.

Core-only `perf_probe` runs with audio draining measured Tetris gameplay at
212.37 frames/s and Saga2 at 161.40 frames/s. These are single-run throughput
checks, not GUI FPS guarantees. A separate 300-frame CLI startup/capture check
took 16.957 s before and 16.585 s after; background-load differences mean this
is only a coarse regression check, not a claimed speedup.

## References and limits

Only hardware documentation and hardware-test descriptions were consulted;
no other emulator implementation was copied.

- [Pan Docs audio details](https://github.com/gbdev/pandocs/blob/master/src/Audio_details.md)
- [Pan Docs audio registers](https://github.com/gbdev/pandocs/blob/master/src/Audio_Registers.md)
- [Pan Docs DIV register](https://github.com/gbdev/pandocs/blob/master/src/Timer_and_Divider_Registers.md)
- [SameSuite CH3 trigger-delay test](https://github.com/LIJI32/SameSuite/blob/master/apu/channel_3/channel_3_delay.asm)
- [SameSuite CH3 live-period test](https://github.com/LIJI32/SameSuite/blob/master/apu/channel_3/channel_3_freq_change_delay.asm)

The attempt to download the public DMG wave-trigger test ROM failed because
shell networking is restricted. External hardware-conformance ROM suites were
therefore not executed. Unit tests and real-ROM smoke are not substitutes for
those suites, listening tests or hardware waveform comparison.

Older CGB silicon revisions, ordinary low-power STOP behavior, sub-M-cycle CPU
bus strobes and unusual stack/opcode accesses mapped onto audio registers are
not claimed cycle-exact by this update. Envelope quirks and analog/band-limited
output improvements remain separate work. GUI executables were not rebuilt.

## Delivery

- Updated binary: `dist/kokura-cli.exe`.
- SHA-256: `3DEE4B0DDAAF5368DE0045EB48953D801460E293009005CA660ABDF3484E9BCC`.
- Previous binary retained at `dist/kokura_cli_20261113_audio/kokura-cli.exe`.
- Compact run reports and pre-change source snapshots are under
  `verification/audio_timing_20261114/`. Generated WAVs and the temporary
  ROM-containing state fixture were removed after validation.

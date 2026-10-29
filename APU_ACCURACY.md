# APU accuracy update: 2026-10-29

The core and CLI now include these independently implemented corrections:

- Pulse retrigger preserves the duty sequencer phase. Only the first duty step
  after APU power-up is suppressed.
- Sweep pace zero does not update pitch. Period writes do not overwrite the
  trigger-latched sweep shadow, and a sweep update preserves divider phase.
  A zero shift still performs the overflow check without writing the period.
- The documented zero-pace, increasing-envelope volume-step technique works
  on pulse and noise channels. Other model-dependent zombie cases are unchanged.
- Sound-register reads apply hardware masks. Powered-off register writes are
  ignored, except for DMG length counters; CGB length writes are ignored.
- The DMG/CGB high-pass charge factors account for all 64 T-cycles in one
  65,536 Hz PCM sample. Constants are evaluated at compile time, not per sample.

No dependencies or serialized APU fields were added. The existing state format
is retained. CLI, C API, Python API and private frontends use the shared core.

## Regression coverage

Fourteen focused tests cover phase, sweep, envelopes, masks, power-off behavior,
DC rejection, stereo tone output, and identical traced/fast PCM generation.
Eleven of these tests failed on the previous implementation and pass after the
changes. The existing DC-input test now checks DC rejection rather than treating
constant input as a sustained musical tone.

Run the portable audio tests with:

```sh
cargo test --locked -p kokura-core --lib apu::
```

These are bounded software checks, not physical-hardware or listening proof.
Instruction-boundary timing, model-specific envelope behavior, exact DIV/APU
coupling, long-period noise counters and band-limited synthesis still need work.
This update does not claim complete APU conformance or eliminate every host
audio underrun. See `BINARY_BUILD.json` and `NATIVE_BINARIES.json` for the exact
source and executable validation records.

## References

- [Pan Docs: audio details](https://github.com/gbdev/pandocs/blob/master/src/Audio_details.md)
- [Pan Docs: audio registers](https://github.com/gbdev/pandocs/blob/master/src/Audio_Registers.md)

No emulator implementation code was copied into this update.

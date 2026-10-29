# Publication checks

## 2026-10-29 source update

`cargo test --workspace --locked --offline` passed on Windows x86_64:
256 tests passed, one manual profiling test ignored. This includes 29 APU tests,
14 of which are new accuracy regressions. Optional external-ROM tests can return
early without their inputs; the count does not certify those external scenarios.
No private GUI or ROM data is included. See `APU_ACCURACY.md` for limitations.

The Windows CLI was rebuilt with static CRT and exercised with six synthetic
audio fixtures, six previous-version states, and copied local DMG/CGB ROMs.
Linux x86_64 and both macOS architectures each passed 264 native tests plus
CLI, C API and Python execution/state checks. Binary identities are recorded
in `BINARY_BUILD.json` and `NATIVE_BINARIES.json`.

Scoped rustfmt passes for the changed APU module and its tests. A full workspace
fmt check still reports pre-existing formatting differences in CLI tests,
core examples/tests and debug event comments; those files are not reformatted
by this audio update. Test counts do not certify device playback or hardware.

Workspace test record: September 12, 2026, on Windows x86_64.
The executable build record is identified separately in `BINARY_BUILD.json`.

cargo test --workspace --offline passed: 239 tests reported passed, one manual profiling test ignored. Optional external-ROM probes may return early without their inputs; those scenarios are not verified by this count.

Build caches, private inputs and generated ROMs are excluded from Git. These checks do not cover physical hardware.

## Windows CLI executable distribution - 2026-09-13

A CLI-only Release executable is included at the repository root.
The build uses the locked dependencies, x86_64-pc-windows-msvc and static CRT.
Current binary hashes and executed checks are recorded in `BINARY_BUILD.json`.
Each test record applies to its recorded source and executable hashes.
The independent code is licensed by DAISUKE OBA. Original dependency notices
are bundled; GUI executables and private ROMs/data are not distributed.

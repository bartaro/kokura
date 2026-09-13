# KOKURA Third-Party Notices

For the distributed Windows CLI executable, see [BINARY_NOTICES.md](BINARY_NOTICES.md) and the full notice texts in `licenses/binary-dependencies/` and `licenses/rust-standard-library/`. Existing inventories describe broader or historical source-workspace scopes.

Windows dependency metadata was rechecked on 2026-09-12; see
[`THIRD_PARTY_NOTICES.windows.md`](THIRD_PARTY_NOTICES.windows.md).
The older cross-platform inventory below is retained for reference and is not
the current binary's link map. In addition to metadata expressions, preserve
the bundled font notices in `epaint_default_fonts/fonts/`, including the
Bitstream Vera terms in `Hack-Regular.txt`, OFL, and Ubuntu Font License.

> Generated from `cargo metadata` on 2026-07-25.
> Regenerate with `python tools/generate_third_party_notices.py --write` after changing `Cargo.lock`.

## Scope

KOKURAの自作コードは、リポジトリルートの `LICENSE` に記載されたMITライセンスを対象とします。
このファイルは、KOKURAが依存するCargoレジストリのクレートをMITへ再ライセンスするものではありません。
各依存クレートは、下表のライセンス、著作権表示、NOTICEに従います。

このリポジトリはCargoレジストリの依存ソースをvendorしていません。下表のライセンス/NOTICEファイル名は、
現在のローカルCargoキャッシュで確認したものです。バイナリや自己完結型の配布物を作る場合は、
対応するライセンス本文とNOTICEを配布物へコピーし、変更ファイルへの表示など各ライセンスの条件を満たしてください。

## Direct dependencies

ワークスペースの各クレートから直接参照される外部依存です。`MIT OR Apache-2.0` のような表記は、
依存側が示すライセンス選択肢であり、KOKURA本体のMIT表示を変更するものではありません。

| Package | Version | License expression | Registry license/NOTICE files | Source |
| --- | --- | --- | --- | --- |
| `anyhow` | `1.0.102` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anyhow 1.0.102](https://crates.io/crates/anyhow/1.0.102) |
| `base64` | `0.22.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [base64 0.22.1](https://crates.io/crates/base64/0.22.1) |
| `bincode` | `1.3.3` | `MIT` | `LICENSE.md` | [bincode 1.3.3](https://crates.io/crates/bincode/1.3.3) |
| `bitflags` | `2.11.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bitflags 2.11.0](https://crates.io/crates/bitflags/2.11.0) |
| `clap` | `4.6.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [clap 4.6.0](https://crates.io/crates/clap/4.6.0) |
| `cpal` | `0.16.0` | `Apache-2.0` | `LICENSE` | [cpal 0.16.0](https://crates.io/crates/cpal/0.16.0) |
| `dioxus` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus 0.7.5](https://crates.io/crates/dioxus/0.7.5) |
| `eframe` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [eframe 0.32.3](https://crates.io/crates/eframe/0.32.3) |
| `font8x8` | `0.3.1` | `MIT` | `LICENSE` | [font8x8 0.3.1](https://crates.io/crates/font8x8/0.3.1) |
| `fontdue` | `0.9.3` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [fontdue 0.9.3](https://crates.io/crates/fontdue/0.9.3) |
| `gif` | `0.13.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [gif 0.13.3](https://crates.io/crates/gif/0.13.3) |
| `gilrs` | `0.11.1` | `Apache-2.0/MIT` | (no top-level license file; Cargo metadata expression only) | [gilrs 0.11.1](https://crates.io/crates/gilrs/0.11.1) |
| `image` | `0.25.10` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [image 0.25.10](https://crates.io/crates/image/0.25.10) |
| `parking_lot` | `0.12.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [parking_lot 0.12.5](https://crates.io/crates/parking_lot/0.12.5) |
| `pixels` | `0.15.0` | `MIT` | `LICENSE` | [pixels 0.15.0](https://crates.io/crates/pixels/0.15.0) |
| `pyo3` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3 0.22.6](https://crates.io/crates/pyo3/0.22.6) |
| `rfd` | `0.17.2` | `MIT` | `LICENSE` | [rfd 0.17.2](https://crates.io/crates/rfd/0.17.2) |
| `sdl2` | `0.37.0` | `MIT` | `LICENSE` | [sdl2 0.37.0](https://crates.io/crates/sdl2/0.37.0) |
| `serde` | `1.0.228` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde 1.0.228](https://crates.io/crates/serde/1.0.228) |
| `serde_json` | `1.0.149` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_json 1.0.149](https://crates.io/crates/serde_json/1.0.149) |
| `thiserror` | `2.0.18` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thiserror 2.0.18](https://crates.io/crates/thiserror/2.0.18) |
| `tokio` | `1.51.1` | `MIT` | `LICENSE` | [tokio 1.51.1](https://crates.io/crates/tokio/1.51.1) |
| `winit` | `0.29.15` | `Apache-2.0` | `LICENSE` | [winit 0.29.15](https://crates.io/crates/winit/0.29.15) |
| `zip` | `0.6.6` | `MIT` | `LICENSE` | [zip 0.6.6](https://crates.io/crates/zip/0.6.6) |

## Resolved transitive dependency inventory

Cargoが解決した外部レジストリパッケージは **793件** です。
同じクレート名でも複数バージョンが存在する場合は、バージョンごとに記録します。

| Package | Version | License expression | License/NOTICE files | Source |
| --- | --- | --- | --- | --- |
| `ab_glyph` | `0.2.32` | `Apache-2.0` | `LICENSE` | [ab_glyph 0.2.32](https://crates.io/crates/ab_glyph/0.2.32) |
| `ab_glyph_rasterizer` | `0.1.10` | `Apache-2.0` | `LICENSE` | [ab_glyph_rasterizer 0.1.10](https://crates.io/crates/ab_glyph_rasterizer/0.1.10) |
| `accesskit` | `0.19.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit 0.19.0](https://crates.io/crates/accesskit/0.19.0) |
| `accesskit_atspi_common` | `0.12.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_atspi_common 0.12.0](https://crates.io/crates/accesskit_atspi_common/0.12.0) |
| `accesskit_consumer` | `0.28.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_consumer 0.28.0](https://crates.io/crates/accesskit_consumer/0.28.0) |
| `accesskit_macos` | `0.20.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_macos 0.20.0](https://crates.io/crates/accesskit_macos/0.20.0) |
| `accesskit_unix` | `0.15.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_unix 0.15.0](https://crates.io/crates/accesskit_unix/0.15.0) |
| `accesskit_windows` | `0.27.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_windows 0.27.0](https://crates.io/crates/accesskit_windows/0.27.0) |
| `accesskit_winit` | `0.27.0` | `Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [accesskit_winit 0.27.0](https://crates.io/crates/accesskit_winit/0.27.0) |
| `adler2` | `2.0.1` | `0BSD OR MIT OR Apache-2.0` | `LICENSE-0BSD`, `LICENSE-APACHE`, `LICENSE-MIT` | [adler2 2.0.1](https://crates.io/crates/adler2/2.0.1) |
| `aes` | `0.8.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [aes 0.8.4](https://crates.io/crates/aes/0.8.4) |
| `ahash` | `0.8.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ahash 0.8.12](https://crates.io/crates/ahash/0.8.12) |
| `aho-corasick` | `1.1.4` | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT` | [aho-corasick 1.1.4](https://crates.io/crates/aho-corasick/1.1.4) |
| `allocator-api2` | `0.2.21` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [allocator-api2 0.2.21](https://crates.io/crates/allocator-api2/0.2.21) |
| `alsa` | `0.9.1` | `Apache-2.0/MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [alsa 0.9.1](https://crates.io/crates/alsa/0.9.1) |
| `alsa-sys` | `0.3.1` | `MIT` | `LICENSE` | [alsa-sys 0.3.1](https://crates.io/crates/alsa-sys/0.3.1) |
| `android-activity` | `0.5.2` | `MIT OR Apache-2.0` | `LICENSE` | [android-activity 0.5.2](https://crates.io/crates/android-activity/0.5.2) |
| `android-activity` | `0.6.1` | `MIT OR Apache-2.0` | `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT` | [android-activity 0.6.1](https://crates.io/crates/android-activity/0.6.1) |
| `android-properties` | `0.2.2` | `MIT` | `LICENSE` | [android-properties 0.2.2](https://crates.io/crates/android-properties/0.2.2) |
| `android_system_properties` | `0.1.5` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [android_system_properties 0.1.5](https://crates.io/crates/android_system_properties/0.1.5) |
| `anstream` | `1.0.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anstream 1.0.0](https://crates.io/crates/anstream/1.0.0) |
| `anstyle` | `1.0.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anstyle 1.0.14](https://crates.io/crates/anstyle/1.0.14) |
| `anstyle-parse` | `1.0.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anstyle-parse 1.0.0](https://crates.io/crates/anstyle-parse/1.0.0) |
| `anstyle-query` | `1.1.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anstyle-query 1.1.5](https://crates.io/crates/anstyle-query/1.1.5) |
| `anstyle-wincon` | `3.0.11` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anstyle-wincon 3.0.11](https://crates.io/crates/anstyle-wincon/3.0.11) |
| `anyhow` | `1.0.102` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [anyhow 1.0.102](https://crates.io/crates/anyhow/1.0.102) |
| `arboard` | `3.6.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE.txt`, `LICENSE-MIT.txt` | [arboard 3.6.1](https://crates.io/crates/arboard/3.6.1) |
| `arrayref` | `0.3.9` | `BSD-2-Clause` | `LICENSE` | [arrayref 0.3.9](https://crates.io/crates/arrayref/0.3.9) |
| `arrayvec` | `0.7.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [arrayvec 0.7.6](https://crates.io/crates/arrayvec/0.7.6) |
| `as-raw-xcb-connection` | `1.0.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [as-raw-xcb-connection 1.0.1](https://crates.io/crates/as-raw-xcb-connection/1.0.1) |
| `ash` | `0.37.3+1.3.251` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ash 0.37.3+1.3.251](https://crates.io/crates/ash/0.37.3+1.3.251) |
| `ash` | `0.38.0+1.3.281` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ash 0.38.0+1.3.281](https://crates.io/crates/ash/0.38.0+1.3.281) |
| `async-broadcast` | `0.7.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-broadcast 0.7.2](https://crates.io/crates/async-broadcast/0.7.2) |
| `async-channel` | `2.5.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-channel 2.5.0](https://crates.io/crates/async-channel/2.5.0) |
| `async-executor` | `1.14.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-executor 1.14.0](https://crates.io/crates/async-executor/1.14.0) |
| `async-io` | `2.6.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-io 2.6.0](https://crates.io/crates/async-io/2.6.0) |
| `async-lock` | `3.4.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-lock 3.4.2](https://crates.io/crates/async-lock/3.4.2) |
| `async-process` | `2.5.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-process 2.5.0](https://crates.io/crates/async-process/2.5.0) |
| `async-recursion` | `1.1.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-recursion 1.1.1](https://crates.io/crates/async-recursion/1.1.1) |
| `async-signal` | `0.2.14` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-signal 0.2.14](https://crates.io/crates/async-signal/0.2.14) |
| `async-task` | `4.7.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-task 4.7.1](https://crates.io/crates/async-task/4.7.1) |
| `async-trait` | `0.1.89` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [async-trait 0.1.89](https://crates.io/crates/async-trait/0.1.89) |
| `atk` | `0.18.2` | `MIT` | `LICENSE` | [atk 0.18.2](https://crates.io/crates/atk/0.18.2) |
| `atk-sys` | `0.18.2` | `MIT` | `LICENSE` | [atk-sys 0.18.2](https://crates.io/crates/atk-sys/0.18.2) |
| `atomic-waker` | `1.1.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-THIRD-PARTY` | [atomic-waker 1.1.2](https://crates.io/crates/atomic-waker/1.1.2) |
| `atspi` | `0.25.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE2.txt`, `LICENSE-MIT.txt` | [atspi 0.25.0](https://crates.io/crates/atspi/0.25.0) |
| `atspi-common` | `0.9.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE2.txt`, `LICENSE-MIT.txt` | [atspi-common 0.9.0](https://crates.io/crates/atspi-common/0.9.0) |
| `atspi-connection` | `0.9.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE2.txt`, `LICENSE-MIT.txt` | [atspi-connection 0.9.0](https://crates.io/crates/atspi-connection/0.9.0) |
| `atspi-proxies` | `0.9.0` | `Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [atspi-proxies 0.9.0](https://crates.io/crates/atspi-proxies/0.9.0) |
| `autocfg` | `1.5.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [autocfg 1.5.0](https://crates.io/crates/autocfg/1.5.0) |
| `base64` | `0.22.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [base64 0.22.1](https://crates.io/crates/base64/0.22.1) |
| `base64ct` | `1.8.3` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [base64ct 1.8.3](https://crates.io/crates/base64ct/1.8.3) |
| `bincode` | `1.3.3` | `MIT` | `LICENSE.md` | [bincode 1.3.3](https://crates.io/crates/bincode/1.3.3) |
| `bit-set` | `0.5.3` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bit-set 0.5.3](https://crates.io/crates/bit-set/0.5.3) |
| `bit-set` | `0.8.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [bit-set 0.8.0](https://crates.io/crates/bit-set/0.8.0) |
| `bit-vec` | `0.6.3` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bit-vec 0.6.3](https://crates.io/crates/bit-vec/0.6.3) |
| `bit-vec` | `0.8.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [bit-vec 0.8.0](https://crates.io/crates/bit-vec/0.8.0) |
| `bitflags` | `1.3.2` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bitflags 1.3.2](https://crates.io/crates/bitflags/1.3.2) |
| `bitflags` | `2.11.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bitflags 2.11.0](https://crates.io/crates/bitflags/2.11.0) |
| `block` | `0.1.6` | `MIT` | (no top-level license file; Cargo metadata expression only) | [block 0.1.6](https://crates.io/crates/block/0.1.6) |
| `block-buffer` | `0.10.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [block-buffer 0.10.4](https://crates.io/crates/block-buffer/0.10.4) |
| `block-sys` | `0.2.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [block-sys 0.2.1](https://crates.io/crates/block-sys/0.2.1) |
| `block2` | `0.3.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [block2 0.3.0](https://crates.io/crates/block2/0.3.0) |
| `block2` | `0.5.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [block2 0.5.1](https://crates.io/crates/block2/0.5.1) |
| `block2` | `0.6.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [block2 0.6.2](https://crates.io/crates/block2/0.6.2) |
| `blocking` | `1.6.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [blocking 1.6.2](https://crates.io/crates/blocking/1.6.2) |
| `bumpalo` | `3.20.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bumpalo 3.20.2](https://crates.io/crates/bumpalo/3.20.2) |
| `bytemuck` | `1.25.0` | `Zlib OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [bytemuck 1.25.0](https://crates.io/crates/bytemuck/1.25.0) |
| `bytemuck_derive` | `1.10.2` | `Zlib OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [bytemuck_derive 1.10.2](https://crates.io/crates/bytemuck_derive/1.10.2) |
| `byteorder` | `1.5.0` | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT` | [byteorder 1.5.0](https://crates.io/crates/byteorder/1.5.0) |
| `byteorder-lite` | `0.1.0` | `Unlicense OR MIT` | `LICENSE-MIT` | [byteorder-lite 0.1.0](https://crates.io/crates/byteorder-lite/0.1.0) |
| `bytes` | `1.11.1` | `MIT` | `LICENSE` | [bytes 1.11.1](https://crates.io/crates/bytes/1.11.1) |
| `bzip2` | `0.4.4` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bzip2 0.4.4](https://crates.io/crates/bzip2/0.4.4) |
| `bzip2-sys` | `0.1.13+1.0.8` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [bzip2-sys 0.1.13+1.0.8](https://crates.io/crates/bzip2-sys/0.1.13+1.0.8) |
| `cairo-rs` | `0.18.5` | `MIT` | `LICENSE` | [cairo-rs 0.18.5](https://crates.io/crates/cairo-rs/0.18.5) |
| `cairo-sys-rs` | `0.18.2` | `MIT` | `LICENSE` | [cairo-sys-rs 0.18.2](https://crates.io/crates/cairo-sys-rs/0.18.2) |
| `calloop` | `0.12.4` | `MIT` | `LICENSE.txt` | [calloop 0.12.4](https://crates.io/crates/calloop/0.12.4) |
| `calloop` | `0.13.0` | `MIT` | `LICENSE.txt` | [calloop 0.13.0](https://crates.io/crates/calloop/0.13.0) |
| `calloop` | `0.14.4` | `MIT` | `LICENSE.txt` | [calloop 0.14.4](https://crates.io/crates/calloop/0.14.4) |
| `calloop-wayland-source` | `0.2.0` | `MIT` | `LICENSE.txt` | [calloop-wayland-source 0.2.0](https://crates.io/crates/calloop-wayland-source/0.2.0) |
| `calloop-wayland-source` | `0.3.0` | `MIT` | `LICENSE.txt` | [calloop-wayland-source 0.3.0](https://crates.io/crates/calloop-wayland-source/0.3.0) |
| `calloop-wayland-source` | `0.4.1` | `MIT` | `LICENSE.txt` | [calloop-wayland-source 0.4.1](https://crates.io/crates/calloop-wayland-source/0.4.1) |
| `cc` | `1.2.58` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cc 1.2.58](https://crates.io/crates/cc/1.2.58) |
| `cesu8` | `1.1.0` | `Apache-2.0/MIT` | (no top-level license file; Cargo metadata expression only) | [cesu8 1.1.0](https://crates.io/crates/cesu8/1.1.0) |
| `cfb` | `0.7.3` | `MIT` | `LICENSE` | [cfb 0.7.3](https://crates.io/crates/cfb/0.7.3) |
| `cfg-expr` | `0.15.8` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cfg-expr 0.15.8](https://crates.io/crates/cfg-expr/0.15.8) |
| `cfg-if` | `1.0.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cfg-if 1.0.4](https://crates.io/crates/cfg-if/1.0.4) |
| `cfg_aliases` | `0.1.1` | `MIT` | `LICENSE`, `NOTICES.md` | [cfg_aliases 0.1.1](https://crates.io/crates/cfg_aliases/0.1.1) |
| `cfg_aliases` | `0.2.1` | `MIT` | `LICENSE`, `NOTICES.md` | [cfg_aliases 0.2.1](https://crates.io/crates/cfg_aliases/0.2.1) |
| `cgl` | `0.3.2` | `MIT / Apache-2.0` | `COPYING`, `LICENSE-APACHE`, `LICENSE-MIT` | [cgl 0.3.2](https://crates.io/crates/cgl/0.3.2) |
| `cipher` | `0.4.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cipher 0.4.4](https://crates.io/crates/cipher/0.4.4) |
| `clap` | `4.6.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [clap 4.6.0](https://crates.io/crates/clap/4.6.0) |
| `clap_builder` | `4.6.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [clap_builder 4.6.0](https://crates.io/crates/clap_builder/4.6.0) |
| `clap_derive` | `4.6.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [clap_derive 4.6.0](https://crates.io/crates/clap_derive/4.6.0) |
| `clap_lex` | `1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [clap_lex 1.1.0](https://crates.io/crates/clap_lex/1.1.0) |
| `clipboard-win` | `5.4.1` | `BSL-1.0` | (no top-level license file; Cargo metadata expression only) | [clipboard-win 5.4.1](https://crates.io/crates/clipboard-win/5.4.1) |
| `cmake` | `0.1.58` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cmake 0.1.58](https://crates.io/crates/cmake/0.1.58) |
| `cocoa` | `0.26.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cocoa 0.26.1](https://crates.io/crates/cocoa/0.26.1) |
| `cocoa-foundation` | `0.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cocoa-foundation 0.2.1](https://crates.io/crates/cocoa-foundation/0.2.1) |
| `codespan-reporting` | `0.11.1` | `Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [codespan-reporting 0.11.1](https://crates.io/crates/codespan-reporting/0.11.1) |
| `codespan-reporting` | `0.12.0` | `Apache-2.0` | `LICENSE` | [codespan-reporting 0.12.0](https://crates.io/crates/codespan-reporting/0.12.0) |
| `color_quant` | `1.1.0` | `MIT` | `LICENSE` | [color_quant 1.1.0](https://crates.io/crates/color_quant/1.1.0) |
| `colorchoice` | `1.0.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [colorchoice 1.0.5](https://crates.io/crates/colorchoice/1.0.5) |
| `com` | `0.6.0` | `MIT` | `LICENSE` | [com 0.6.0](https://crates.io/crates/com/0.6.0) |
| `com_macros` | `0.6.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [com_macros 0.6.0](https://crates.io/crates/com_macros/0.6.0) |
| `com_macros_support` | `0.6.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [com_macros_support 0.6.0](https://crates.io/crates/com_macros_support/0.6.0) |
| `combine` | `4.6.7` | `MIT` | `LICENSE` | [combine 4.6.7](https://crates.io/crates/combine/4.6.7) |
| `concurrent-queue` | `2.5.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [concurrent-queue 2.5.0](https://crates.io/crates/concurrent-queue/2.5.0) |
| `const-serialize` | `0.7.2` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [const-serialize 0.7.2](https://crates.io/crates/const-serialize/0.7.2) |
| `const-serialize` | `0.8.0-alpha.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [const-serialize 0.8.0-alpha.0](https://crates.io/crates/const-serialize/0.8.0-alpha.0) |
| `const-serialize-macro` | `0.7.2` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [const-serialize-macro 0.7.2](https://crates.io/crates/const-serialize-macro/0.7.2) |
| `const-serialize-macro` | `0.8.0-alpha.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [const-serialize-macro 0.8.0-alpha.0](https://crates.io/crates/const-serialize-macro/0.8.0-alpha.0) |
| `const_format` | `0.2.35` | `Zlib` | `LICENSE-ZLIB.md` | [const_format 0.2.35](https://crates.io/crates/const_format/0.2.35) |
| `const_format_proc_macros` | `0.2.34` | `Zlib` | `LICENSE-ZLIB.md` | [const_format_proc_macros 0.2.34](https://crates.io/crates/const_format_proc_macros/0.2.34) |
| `constant_time_eq` | `0.1.5` | `CC0-1.0` | `LICENSE.txt` | [constant_time_eq 0.1.5](https://crates.io/crates/constant_time_eq/0.1.5) |
| `convert_case` | `0.4.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [convert_case 0.4.0](https://crates.io/crates/convert_case/0.4.0) |
| `convert_case` | `0.8.0` | `MIT` | `LICENSE` | [convert_case 0.8.0](https://crates.io/crates/convert_case/0.8.0) |
| `cookie` | `0.18.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cookie 0.18.1](https://crates.io/crates/cookie/0.18.1) |
| `core-foundation` | `0.10.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-foundation 0.10.1](https://crates.io/crates/core-foundation/0.10.1) |
| `core-foundation` | `0.9.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-foundation 0.9.4](https://crates.io/crates/core-foundation/0.9.4) |
| `core-foundation-sys` | `0.8.7` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-foundation-sys 0.8.7](https://crates.io/crates/core-foundation-sys/0.8.7) |
| `core-graphics` | `0.23.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-graphics 0.23.2](https://crates.io/crates/core-graphics/0.23.2) |
| `core-graphics` | `0.24.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-graphics 0.24.0](https://crates.io/crates/core-graphics/0.24.0) |
| `core-graphics` | `0.25.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-graphics 0.25.0](https://crates.io/crates/core-graphics/0.25.0) |
| `core-graphics-types` | `0.1.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-graphics-types 0.1.3](https://crates.io/crates/core-graphics-types/0.1.3) |
| `core-graphics-types` | `0.2.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [core-graphics-types 0.2.0](https://crates.io/crates/core-graphics-types/0.2.0) |
| `coreaudio-rs` | `0.13.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [coreaudio-rs 0.13.0](https://crates.io/crates/coreaudio-rs/0.13.0) |
| `cpal` | `0.16.0` | `Apache-2.0` | `LICENSE` | [cpal 0.16.0](https://crates.io/crates/cpal/0.16.0) |
| `cpufeatures` | `0.2.17` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [cpufeatures 0.2.17](https://crates.io/crates/cpufeatures/0.2.17) |
| `crc32fast` | `1.5.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [crc32fast 1.5.0](https://crates.io/crates/crc32fast/1.5.0) |
| `crossbeam-channel` | `0.5.15` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-THIRD-PARTY` | [crossbeam-channel 0.5.15](https://crates.io/crates/crossbeam-channel/0.5.15) |
| `crossbeam-utils` | `0.8.21` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [crossbeam-utils 0.8.21](https://crates.io/crates/crossbeam-utils/0.8.21) |
| `crunchy` | `0.2.4` | `MIT` | `LICENSE` | [crunchy 0.2.4](https://crates.io/crates/crunchy/0.2.4) |
| `crypto-common` | `0.1.7` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [crypto-common 0.1.7](https://crates.io/crates/crypto-common/0.1.7) |
| `cssparser` | `0.29.6` | `MPL-2.0` | `LICENSE` | [cssparser 0.29.6](https://crates.io/crates/cssparser/0.29.6) |
| `cssparser-macros` | `0.6.1` | `MPL-2.0` | `LICENSE` | [cssparser-macros 0.6.1](https://crates.io/crates/cssparser-macros/0.6.1) |
| `cursor-icon` | `1.2.0` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [cursor-icon 1.2.0](https://crates.io/crates/cursor-icon/1.2.0) |
| `d3d12` | `0.19.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [d3d12 0.19.0](https://crates.io/crates/d3d12/0.19.0) |
| `darling` | `0.21.3` | `MIT` | `LICENSE` | [darling 0.21.3](https://crates.io/crates/darling/0.21.3) |
| `darling_core` | `0.21.3` | `MIT` | `LICENSE` | [darling_core 0.21.3](https://crates.io/crates/darling_core/0.21.3) |
| `darling_macro` | `0.21.3` | `MIT` | `LICENSE` | [darling_macro 0.21.3](https://crates.io/crates/darling_macro/0.21.3) |
| `dasp_sample` | `0.11.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dasp_sample 0.11.0](https://crates.io/crates/dasp_sample/0.11.0) |
| `data-encoding` | `2.10.0` | `MIT` | `LICENSE` | [data-encoding 2.10.0](https://crates.io/crates/data-encoding/2.10.0) |
| `deranged` | `0.5.8` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [deranged 0.5.8](https://crates.io/crates/deranged/0.5.8) |
| `derive_more` | `0.99.20` | `MIT` | `LICENSE` | [derive_more 0.99.20](https://crates.io/crates/derive_more/0.99.20) |
| `digest` | `0.10.7` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [digest 0.10.7](https://crates.io/crates/digest/0.10.7) |
| `dioxus` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus 0.7.5](https://crates.io/crates/dioxus/0.7.5) |
| `dioxus-asset-resolver` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-asset-resolver 0.7.4](https://crates.io/crates/dioxus-asset-resolver/0.7.4) |
| `dioxus-cli-config` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-cli-config 0.7.4](https://crates.io/crates/dioxus-cli-config/0.7.4) |
| `dioxus-config-macro` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-config-macro 0.7.4](https://crates.io/crates/dioxus-config-macro/0.7.4) |
| `dioxus-config-macros` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-config-macros 0.7.4](https://crates.io/crates/dioxus-config-macros/0.7.4) |
| `dioxus-core` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-core 0.7.4](https://crates.io/crates/dioxus-core/0.7.4) |
| `dioxus-core-macro` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-core-macro 0.7.4](https://crates.io/crates/dioxus-core-macro/0.7.4) |
| `dioxus-core-types` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-core-types 0.7.4](https://crates.io/crates/dioxus-core-types/0.7.4) |
| `dioxus-desktop` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-desktop 0.7.5](https://crates.io/crates/dioxus-desktop/0.7.5) |
| `dioxus-devtools` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-devtools 0.7.4](https://crates.io/crates/dioxus-devtools/0.7.4) |
| `dioxus-devtools-types` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-devtools-types 0.7.4](https://crates.io/crates/dioxus-devtools-types/0.7.4) |
| `dioxus-document` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-document 0.7.4](https://crates.io/crates/dioxus-document/0.7.4) |
| `dioxus-history` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-history 0.7.4](https://crates.io/crates/dioxus-history/0.7.4) |
| `dioxus-hooks` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-hooks 0.7.4](https://crates.io/crates/dioxus-hooks/0.7.4) |
| `dioxus-html` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-html 0.7.5](https://crates.io/crates/dioxus-html/0.7.5) |
| `dioxus-html-internal-macro` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-html-internal-macro 0.7.4](https://crates.io/crates/dioxus-html-internal-macro/0.7.4) |
| `dioxus-interpreter-js` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-interpreter-js 0.7.4](https://crates.io/crates/dioxus-interpreter-js/0.7.4) |
| `dioxus-logger` | `0.7.4` | `MIT` | (no top-level license file; Cargo metadata expression only) | [dioxus-logger 0.7.4](https://crates.io/crates/dioxus-logger/0.7.4) |
| `dioxus-rsx` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-rsx 0.7.4](https://crates.io/crates/dioxus-rsx/0.7.4) |
| `dioxus-signals` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-signals 0.7.4](https://crates.io/crates/dioxus-signals/0.7.4) |
| `dioxus-stores` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-stores 0.7.4](https://crates.io/crates/dioxus-stores/0.7.4) |
| `dioxus-stores-macro` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-stores-macro 0.7.4](https://crates.io/crates/dioxus-stores-macro/0.7.4) |
| `dioxus-web` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [dioxus-web 0.7.5](https://crates.io/crates/dioxus-web/0.7.5) |
| `dirs` | `6.0.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [dirs 6.0.0](https://crates.io/crates/dirs/6.0.0) |
| `dirs-sys` | `0.5.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [dirs-sys 0.5.0](https://crates.io/crates/dirs-sys/0.5.0) |
| `dispatch` | `0.2.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [dispatch 0.2.0](https://crates.io/crates/dispatch/0.2.0) |
| `dispatch2` | `0.3.1` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [dispatch2 0.3.1](https://crates.io/crates/dispatch2/0.3.1) |
| `displaydoc` | `0.2.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [displaydoc 0.2.5](https://crates.io/crates/displaydoc/0.2.5) |
| `dlib` | `0.5.3` | `MIT` | `LICENSE.txt` | [dlib 0.5.3](https://crates.io/crates/dlib/0.5.3) |
| `dlopen2` | `0.8.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [dlopen2 0.8.2](https://crates.io/crates/dlopen2/0.8.2) |
| `dlopen2_derive` | `0.4.3` | `MIT` | (no top-level license file; Cargo metadata expression only) | [dlopen2_derive 0.4.3](https://crates.io/crates/dlopen2_derive/0.4.3) |
| `document-features` | `0.2.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [document-features 0.2.12](https://crates.io/crates/document-features/0.2.12) |
| `downcast-rs` | `1.2.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [downcast-rs 1.2.1](https://crates.io/crates/downcast-rs/1.2.1) |
| `dpi` | `0.1.2` | `Apache-2.0 AND MIT` | `LICENSE`, `LICENSE-LIBM-MIT` | [dpi 0.1.2](https://crates.io/crates/dpi/0.1.2) |
| `dtoa` | `1.0.11` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [dtoa 1.0.11](https://crates.io/crates/dtoa/1.0.11) |
| `dtoa-short` | `0.3.5` | `MPL-2.0` | `LICENSE` | [dtoa-short 0.3.5](https://crates.io/crates/dtoa-short/0.3.5) |
| `dunce` | `1.0.5` | `CC0-1.0 OR MIT-0 OR Apache-2.0` | `LICENSE` | [dunce 1.0.5](https://crates.io/crates/dunce/1.0.5) |
| `ecolor` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ecolor 0.32.3](https://crates.io/crates/ecolor/0.32.3) |
| `eframe` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [eframe 0.32.3](https://crates.io/crates/eframe/0.32.3) |
| `egui` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [egui 0.32.3](https://crates.io/crates/egui/0.32.3) |
| `egui-wgpu` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [egui-wgpu 0.32.3](https://crates.io/crates/egui-wgpu/0.32.3) |
| `egui-winit` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [egui-winit 0.32.3](https://crates.io/crates/egui-winit/0.32.3) |
| `egui_glow` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [egui_glow 0.32.3](https://crates.io/crates/egui_glow/0.32.3) |
| `emath` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [emath 0.32.3](https://crates.io/crates/emath/0.32.3) |
| `endi` | `1.1.1` | `MIT` | `LICENSE-MIT` | [endi 1.1.1](https://crates.io/crates/endi/1.1.1) |
| `enumflags2` | `0.7.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [enumflags2 0.7.12](https://crates.io/crates/enumflags2/0.7.12) |
| `enumflags2_derive` | `0.7.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [enumflags2_derive 0.7.12](https://crates.io/crates/enumflags2_derive/0.7.12) |
| `enumset` | `1.1.10` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [enumset 1.1.10](https://crates.io/crates/enumset/1.1.10) |
| `enumset_derive` | `0.14.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [enumset_derive 0.14.0](https://crates.io/crates/enumset_derive/0.14.0) |
| `epaint` | `0.32.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [epaint 0.32.3](https://crates.io/crates/epaint/0.32.3) |
| `epaint_default_fonts` | `0.32.3` | `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0` | (no top-level license file; Cargo metadata expression only) | [epaint_default_fonts 0.32.3](https://crates.io/crates/epaint_default_fonts/0.32.3) |
| `equivalent` | `1.0.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [equivalent 1.0.2](https://crates.io/crates/equivalent/1.0.2) |
| `errno` | `0.3.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [errno 0.3.14](https://crates.io/crates/errno/0.3.14) |
| `error-code` | `3.3.2` | `BSL-1.0` | `LICENSE` | [error-code 3.3.2](https://crates.io/crates/error-code/3.3.2) |
| `euclid` | `0.22.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [euclid 0.22.14](https://crates.io/crates/euclid/0.22.14) |
| `event-listener` | `5.4.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [event-listener 5.4.1](https://crates.io/crates/event-listener/5.4.1) |
| `event-listener-strategy` | `0.5.4` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [event-listener-strategy 0.5.4](https://crates.io/crates/event-listener-strategy/0.5.4) |
| `fastrand` | `2.4.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [fastrand 2.4.1](https://crates.io/crates/fastrand/2.4.1) |
| `fax` | `0.2.6` | `MIT` | (no top-level license file; Cargo metadata expression only) | [fax 0.2.6](https://crates.io/crates/fax/0.2.6) |
| `fax_derive` | `0.2.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [fax_derive 0.2.0](https://crates.io/crates/fax_derive/0.2.0) |
| `fdeflate` | `0.3.7` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [fdeflate 0.3.7](https://crates.io/crates/fdeflate/0.3.7) |
| `field-offset` | `0.3.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [field-offset 0.3.6](https://crates.io/crates/field-offset/0.3.6) |
| `find-msvc-tools` | `0.1.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [find-msvc-tools 0.1.9](https://crates.io/crates/find-msvc-tools/0.1.9) |
| `flate2` | `1.1.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [flate2 1.1.9](https://crates.io/crates/flate2/1.1.9) |
| `fnv` | `1.0.7` | `Apache-2.0 / MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [fnv 1.0.7](https://crates.io/crates/fnv/1.0.7) |
| `foldhash` | `0.1.5` | `Zlib` | `LICENSE` | [foldhash 0.1.5](https://crates.io/crates/foldhash/0.1.5) |
| `font8x8` | `0.3.1` | `MIT` | `LICENSE` | [font8x8 0.3.1](https://crates.io/crates/font8x8/0.3.1) |
| `fontdue` | `0.9.3` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [fontdue 0.9.3](https://crates.io/crates/fontdue/0.9.3) |
| `foreign-types` | `0.3.2` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [foreign-types 0.3.2](https://crates.io/crates/foreign-types/0.3.2) |
| `foreign-types` | `0.5.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [foreign-types 0.5.0](https://crates.io/crates/foreign-types/0.5.0) |
| `foreign-types-macros` | `0.2.3` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [foreign-types-macros 0.2.3](https://crates.io/crates/foreign-types-macros/0.2.3) |
| `foreign-types-shared` | `0.1.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [foreign-types-shared 0.1.1](https://crates.io/crates/foreign-types-shared/0.1.1) |
| `foreign-types-shared` | `0.3.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [foreign-types-shared 0.3.1](https://crates.io/crates/foreign-types-shared/0.3.1) |
| `form_urlencoded` | `1.2.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [form_urlencoded 1.2.2](https://crates.io/crates/form_urlencoded/1.2.2) |
| `futf` | `0.1.5` | `MIT / Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futf 0.1.5](https://crates.io/crates/futf/0.1.5) |
| `futures-channel` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-channel 0.3.32](https://crates.io/crates/futures-channel/0.3.32) |
| `futures-core` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-core 0.3.32](https://crates.io/crates/futures-core/0.3.32) |
| `futures-executor` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-executor 0.3.32](https://crates.io/crates/futures-executor/0.3.32) |
| `futures-io` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-io 0.3.32](https://crates.io/crates/futures-io/0.3.32) |
| `futures-lite` | `2.6.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-THIRD-PARTY` | [futures-lite 2.6.1](https://crates.io/crates/futures-lite/2.6.1) |
| `futures-macro` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-macro 0.3.32](https://crates.io/crates/futures-macro/0.3.32) |
| `futures-sink` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-sink 0.3.32](https://crates.io/crates/futures-sink/0.3.32) |
| `futures-task` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-task 0.3.32](https://crates.io/crates/futures-task/0.3.32) |
| `futures-util` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [futures-util 0.3.32](https://crates.io/crates/futures-util/0.3.32) |
| `fxhash` | `0.2.1` | `Apache-2.0/MIT` | (no top-level license file; Cargo metadata expression only) | [fxhash 0.2.1](https://crates.io/crates/fxhash/0.2.1) |
| `gdk` | `0.18.2` | `MIT` | `LICENSE` | [gdk 0.18.2](https://crates.io/crates/gdk/0.18.2) |
| `gdk-pixbuf` | `0.18.5` | `MIT` | `LICENSE` | [gdk-pixbuf 0.18.5](https://crates.io/crates/gdk-pixbuf/0.18.5) |
| `gdk-pixbuf-sys` | `0.18.0` | `MIT` | `LICENSE` | [gdk-pixbuf-sys 0.18.0](https://crates.io/crates/gdk-pixbuf-sys/0.18.0) |
| `gdk-sys` | `0.18.2` | `MIT` | `LICENSE` | [gdk-sys 0.18.2](https://crates.io/crates/gdk-sys/0.18.2) |
| `gdkwayland-sys` | `0.18.2` | `MIT` | `LICENSE` | [gdkwayland-sys 0.18.2](https://crates.io/crates/gdkwayland-sys/0.18.2) |
| `gdkx11-sys` | `0.18.2` | `MIT` | `LICENSE` | [gdkx11-sys 0.18.2](https://crates.io/crates/gdkx11-sys/0.18.2) |
| `generational-box` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [generational-box 0.7.4](https://crates.io/crates/generational-box/0.7.4) |
| `generic-array` | `0.14.7` | `MIT` | `LICENSE` | [generic-array 0.14.7](https://crates.io/crates/generic-array/0.14.7) |
| `gethostname` | `1.1.0` | `Apache-2.0` | `LICENSE` | [gethostname 1.1.0](https://crates.io/crates/gethostname/1.1.0) |
| `getrandom` | `0.1.16` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [getrandom 0.1.16](https://crates.io/crates/getrandom/0.1.16) |
| `getrandom` | `0.2.17` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [getrandom 0.2.17](https://crates.io/crates/getrandom/0.2.17) |
| `getrandom` | `0.3.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [getrandom 0.3.4](https://crates.io/crates/getrandom/0.3.4) |
| `gif` | `0.13.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [gif 0.13.3](https://crates.io/crates/gif/0.13.3) |
| `gilrs` | `0.11.1` | `Apache-2.0/MIT` | (no top-level license file; Cargo metadata expression only) | [gilrs 0.11.1](https://crates.io/crates/gilrs/0.11.1) |
| `gilrs-core` | `0.6.7` | `Apache-2.0/MIT` | (no top-level license file; Cargo metadata expression only) | [gilrs-core 0.6.7](https://crates.io/crates/gilrs-core/0.6.7) |
| `gio` | `0.18.4` | `MIT` | `LICENSE` | [gio 0.18.4](https://crates.io/crates/gio/0.18.4) |
| `gio-sys` | `0.18.1` | `MIT` | `LICENSE` | [gio-sys 0.18.1](https://crates.io/crates/gio-sys/0.18.1) |
| `gl_generator` | `0.14.0` | `Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gl_generator 0.14.0](https://crates.io/crates/gl_generator/0.14.0) |
| `glib` | `0.18.5` | `MIT` | `LICENSE` | [glib 0.18.5](https://crates.io/crates/glib/0.18.5) |
| `glib-macros` | `0.18.5` | `MIT` | `LICENSE` | [glib-macros 0.18.5](https://crates.io/crates/glib-macros/0.18.5) |
| `glib-sys` | `0.18.1` | `MIT` | `LICENSE` | [glib-sys 0.18.1](https://crates.io/crates/glib-sys/0.18.1) |
| `global-hotkey` | `0.7.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.spdx` | [global-hotkey 0.7.0](https://crates.io/crates/global-hotkey/0.7.0) |
| `gloo-timers` | `0.3.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gloo-timers 0.3.0](https://crates.io/crates/gloo-timers/0.3.0) |
| `glow` | `0.13.1` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [glow 0.13.1](https://crates.io/crates/glow/0.13.1) |
| `glow` | `0.16.0` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [glow 0.16.0](https://crates.io/crates/glow/0.16.0) |
| `glutin` | `0.32.3` | `Apache-2.0` | `LICENSE` | [glutin 0.32.3](https://crates.io/crates/glutin/0.32.3) |
| `glutin-winit` | `0.5.0` | `MIT` | `LICENSE` | [glutin-winit 0.5.0](https://crates.io/crates/glutin-winit/0.5.0) |
| `glutin_egl_sys` | `0.7.1` | `Apache-2.0` | `LICENSE` | [glutin_egl_sys 0.7.1](https://crates.io/crates/glutin_egl_sys/0.7.1) |
| `glutin_glx_sys` | `0.6.1` | `Apache-2.0` | `LICENSE` | [glutin_glx_sys 0.6.1](https://crates.io/crates/glutin_glx_sys/0.6.1) |
| `glutin_wgl_sys` | `0.5.0` | `Apache-2.0` | `LICENSE` | [glutin_wgl_sys 0.5.0](https://crates.io/crates/glutin_wgl_sys/0.5.0) |
| `glutin_wgl_sys` | `0.6.1` | `Apache-2.0` | `LICENSE` | [glutin_wgl_sys 0.6.1](https://crates.io/crates/glutin_wgl_sys/0.6.1) |
| `gobject-sys` | `0.18.0` | `MIT` | `LICENSE` | [gobject-sys 0.18.0](https://crates.io/crates/gobject-sys/0.18.0) |
| `gpu-alloc` | `0.6.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-alloc 0.6.0](https://crates.io/crates/gpu-alloc/0.6.0) |
| `gpu-alloc-types` | `0.3.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-alloc-types 0.3.0](https://crates.io/crates/gpu-alloc-types/0.3.0) |
| `gpu-allocator` | `0.25.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [gpu-allocator 0.25.0](https://crates.io/crates/gpu-allocator/0.25.0) |
| `gpu-allocator` | `0.27.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [gpu-allocator 0.27.0](https://crates.io/crates/gpu-allocator/0.27.0) |
| `gpu-descriptor` | `0.2.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-descriptor 0.2.4](https://crates.io/crates/gpu-descriptor/0.2.4) |
| `gpu-descriptor` | `0.3.2` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-descriptor 0.3.2](https://crates.io/crates/gpu-descriptor/0.3.2) |
| `gpu-descriptor-types` | `0.1.2` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-descriptor-types 0.1.2](https://crates.io/crates/gpu-descriptor-types/0.1.2) |
| `gpu-descriptor-types` | `0.2.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [gpu-descriptor-types 0.2.0](https://crates.io/crates/gpu-descriptor-types/0.2.0) |
| `gtk` | `0.18.2` | `MIT` | `LICENSE` | [gtk 0.18.2](https://crates.io/crates/gtk/0.18.2) |
| `gtk-sys` | `0.18.2` | `MIT` | `LICENSE` | [gtk-sys 0.18.2](https://crates.io/crates/gtk-sys/0.18.2) |
| `gtk3-macros` | `0.18.2` | `MIT` | `LICENSE` | [gtk3-macros 0.18.2](https://crates.io/crates/gtk3-macros/0.18.2) |
| `half` | `2.7.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [half 2.7.1](https://crates.io/crates/half/2.7.1) |
| `hashbrown` | `0.14.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hashbrown 0.14.5](https://crates.io/crates/hashbrown/0.14.5) |
| `hashbrown` | `0.15.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hashbrown 0.15.5](https://crates.io/crates/hashbrown/0.15.5) |
| `hashbrown` | `0.16.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hashbrown 0.16.1](https://crates.io/crates/hashbrown/0.16.1) |
| `hassle-rs` | `0.11.0` | `MIT` | `LICENSE` | [hassle-rs 0.11.0](https://crates.io/crates/hassle-rs/0.11.0) |
| `heck` | `0.4.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [heck 0.4.1](https://crates.io/crates/heck/0.4.1) |
| `heck` | `0.5.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [heck 0.5.0](https://crates.io/crates/heck/0.5.0) |
| `hermit-abi` | `0.5.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hermit-abi 0.5.2](https://crates.io/crates/hermit-abi/0.5.2) |
| `hex` | `0.4.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hex 0.4.3](https://crates.io/crates/hex/0.4.3) |
| `hexf-parse` | `0.2.1` | `CC0-1.0` | (no top-level license file; Cargo metadata expression only) | [hexf-parse 0.2.1](https://crates.io/crates/hexf-parse/0.2.1) |
| `hmac` | `0.12.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [hmac 0.12.1](https://crates.io/crates/hmac/0.12.1) |
| `html5ever` | `0.29.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [html5ever 0.29.1](https://crates.io/crates/html5ever/0.29.1) |
| `http` | `1.4.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [http 1.4.0](https://crates.io/crates/http/1.4.0) |
| `httparse` | `1.10.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [httparse 1.10.1](https://crates.io/crates/httparse/1.10.1) |
| `icrate` | `0.0.4` | `MIT` | (no top-level license file; Cargo metadata expression only) | [icrate 0.0.4](https://crates.io/crates/icrate/0.0.4) |
| `icu_collections` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_collections 2.2.0](https://crates.io/crates/icu_collections/2.2.0) |
| `icu_locale_core` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_locale_core 2.2.0](https://crates.io/crates/icu_locale_core/2.2.0) |
| `icu_normalizer` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_normalizer 2.2.0](https://crates.io/crates/icu_normalizer/2.2.0) |
| `icu_normalizer_data` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_normalizer_data 2.2.0](https://crates.io/crates/icu_normalizer_data/2.2.0) |
| `icu_properties` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_properties 2.2.0](https://crates.io/crates/icu_properties/2.2.0) |
| `icu_properties_data` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_properties_data 2.2.0](https://crates.io/crates/icu_properties_data/2.2.0) |
| `icu_provider` | `2.2.0` | `Unicode-3.0` | `LICENSE` | [icu_provider 2.2.0](https://crates.io/crates/icu_provider/2.2.0) |
| `ident_case` | `1.0.1` | `MIT/Apache-2.0` | `LICENSE` | [ident_case 1.0.1](https://crates.io/crates/ident_case/1.0.1) |
| `idna` | `1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [idna 1.1.0](https://crates.io/crates/idna/1.1.0) |
| `idna_adapter` | `1.2.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [idna_adapter 1.2.1](https://crates.io/crates/idna_adapter/1.2.1) |
| `image` | `0.25.10` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [image 0.25.10](https://crates.io/crates/image/0.25.10) |
| `indexmap` | `2.13.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [indexmap 2.13.1](https://crates.io/crates/indexmap/2.13.1) |
| `indoc` | `2.0.7` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [indoc 2.0.7](https://crates.io/crates/indoc/2.0.7) |
| `infer` | `0.19.0` | `MIT` | `LICENSE` | [infer 0.19.0](https://crates.io/crates/infer/0.19.0) |
| `inotify` | `0.11.1` | `ISC` | `LICENSE` | [inotify 0.11.1](https://crates.io/crates/inotify/0.11.1) |
| `inotify-sys` | `0.1.5` | `ISC` | `LICENSE` | [inotify-sys 0.1.5](https://crates.io/crates/inotify-sys/0.1.5) |
| `inout` | `0.1.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [inout 0.1.4](https://crates.io/crates/inout/0.1.4) |
| `is_terminal_polyfill` | `1.70.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [is_terminal_polyfill 1.70.2](https://crates.io/crates/is_terminal_polyfill/1.70.2) |
| `itoa` | `1.0.18` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [itoa 1.0.18](https://crates.io/crates/itoa/1.0.18) |
| `javascriptcore-rs` | `1.1.2` | `MIT` | `LICENSE` | [javascriptcore-rs 1.1.2](https://crates.io/crates/javascriptcore-rs/1.1.2) |
| `javascriptcore-rs-sys` | `1.1.1` | `MIT` | `LICENSE` | [javascriptcore-rs-sys 1.1.1](https://crates.io/crates/javascriptcore-rs-sys/1.1.1) |
| `jni` | `0.21.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [jni 0.21.1](https://crates.io/crates/jni/0.21.1) |
| `jni` | `0.22.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [jni 0.22.4](https://crates.io/crates/jni/0.22.4) |
| `jni-macros` | `0.22.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [jni-macros 0.22.4](https://crates.io/crates/jni-macros/0.22.4) |
| `jni-sys` | `0.3.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [jni-sys 0.3.1](https://crates.io/crates/jni-sys/0.3.1) |
| `jni-sys` | `0.4.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [jni-sys 0.4.1](https://crates.io/crates/jni-sys/0.4.1) |
| `jni-sys-macros` | `0.4.1` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [jni-sys-macros 0.4.1](https://crates.io/crates/jni-sys-macros/0.4.1) |
| `jobserver` | `0.1.34` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [jobserver 0.1.34](https://crates.io/crates/jobserver/0.1.34) |
| `js-sys` | `0.3.94` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [js-sys 0.3.94](https://crates.io/crates/js-sys/0.3.94) |
| `keyboard-types` | `0.7.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [keyboard-types 0.7.0](https://crates.io/crates/keyboard-types/0.7.0) |
| `khronos-egl` | `6.0.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [khronos-egl 6.0.0](https://crates.io/crates/khronos-egl/6.0.0) |
| `khronos_api` | `3.1.0` | `Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [khronos_api 3.1.0](https://crates.io/crates/khronos_api/3.1.0) |
| `kuchikiki` | `0.8.8-speedreader` | `MIT` | `LICENSE` | [kuchikiki 0.8.8-speedreader](https://crates.io/crates/kuchikiki/0.8.8-speedreader) |
| `lazy-js-bundle` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [lazy-js-bundle 0.7.4](https://crates.io/crates/lazy-js-bundle/0.7.4) |
| `lazy_static` | `1.5.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [lazy_static 1.5.0](https://crates.io/crates/lazy_static/1.5.0) |
| `libappindicator` | `0.9.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [libappindicator 0.9.0](https://crates.io/crates/libappindicator/0.9.0) |
| `libappindicator-sys` | `0.9.0` | `Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [libappindicator-sys 0.9.0](https://crates.io/crates/libappindicator-sys/0.9.0) |
| `libc` | `0.2.184` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [libc 0.2.184](https://crates.io/crates/libc/0.2.184) |
| `libloading` | `0.7.4` | `ISC` | `LICENSE` | [libloading 0.7.4](https://crates.io/crates/libloading/0.7.4) |
| `libloading` | `0.8.9` | `ISC` | `LICENSE` | [libloading 0.8.9](https://crates.io/crates/libloading/0.8.9) |
| `libm` | `0.2.16` | `MIT` | `LICENSE.txt` | [libm 0.2.16](https://crates.io/crates/libm/0.2.16) |
| `libredox` | `0.1.15` | `MIT` | `LICENSE` | [libredox 0.1.15](https://crates.io/crates/libredox/0.1.15) |
| `libudev-sys` | `0.1.4` | `MIT` | `LICENSE` | [libudev-sys 0.1.4](https://crates.io/crates/libudev-sys/0.1.4) |
| `libxdo` | `0.6.0` | `MIT` | `LICENSE` | [libxdo 0.6.0](https://crates.io/crates/libxdo/0.6.0) |
| `libxdo-sys` | `0.11.0` | `MIT` | `LICENSE` | [libxdo-sys 0.11.0](https://crates.io/crates/libxdo-sys/0.11.0) |
| `linux-raw-sys` | `0.12.1` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [linux-raw-sys 0.12.1](https://crates.io/crates/linux-raw-sys/0.12.1) |
| `linux-raw-sys` | `0.4.15` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [linux-raw-sys 0.4.15](https://crates.io/crates/linux-raw-sys/0.4.15) |
| `litemap` | `0.8.2` | `Unicode-3.0` | `LICENSE` | [litemap 0.8.2](https://crates.io/crates/litemap/0.8.2) |
| `litrs` | `1.0.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [litrs 1.0.0](https://crates.io/crates/litrs/1.0.0) |
| `lock_api` | `0.4.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [lock_api 0.4.14](https://crates.io/crates/lock_api/0.4.14) |
| `log` | `0.4.29` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [log 0.4.29](https://crates.io/crates/log/0.4.29) |
| `longest-increasing-subsequence` | `0.1.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [longest-increasing-subsequence 0.1.0](https://crates.io/crates/longest-increasing-subsequence/0.1.0) |
| `mac` | `0.1.1` | `MIT/Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [mac 0.1.1](https://crates.io/crates/mac/0.1.1) |
| `mach2` | `0.4.3` | `BSD-2-Clause OR MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` | [mach2 0.4.3](https://crates.io/crates/mach2/0.4.3) |
| `macro-string` | `0.1.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [macro-string 0.1.4](https://crates.io/crates/macro-string/0.1.4) |
| `malloc_buf` | `0.0.6` | `MIT` | (no top-level license file; Cargo metadata expression only) | [malloc_buf 0.0.6](https://crates.io/crates/malloc_buf/0.0.6) |
| `manganis` | `0.7.5` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [manganis 0.7.5](https://crates.io/crates/manganis/0.7.5) |
| `manganis-core` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [manganis-core 0.7.4](https://crates.io/crates/manganis-core/0.7.4) |
| `manganis-macro` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [manganis-macro 0.7.4](https://crates.io/crates/manganis-macro/0.7.4) |
| `markup5ever` | `0.14.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [markup5ever 0.14.1](https://crates.io/crates/markup5ever/0.14.1) |
| `match_token` | `0.1.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [match_token 0.1.0](https://crates.io/crates/match_token/0.1.0) |
| `matchers` | `0.2.0` | `MIT` | `LICENSE` | [matchers 0.2.0](https://crates.io/crates/matchers/0.2.0) |
| `matches` | `0.1.10` | `MIT` | `LICENSE` | [matches 0.1.10](https://crates.io/crates/matches/0.1.10) |
| `memchr` | `2.8.0` | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT` | [memchr 2.8.0](https://crates.io/crates/memchr/2.8.0) |
| `memfd` | `0.6.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [memfd 0.6.5](https://crates.io/crates/memfd/0.6.5) |
| `memmap2` | `0.9.10` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [memmap2 0.9.10](https://crates.io/crates/memmap2/0.9.10) |
| `memoffset` | `0.9.1` | `MIT` | `LICENSE` | [memoffset 0.9.1](https://crates.io/crates/memoffset/0.9.1) |
| `metal` | `0.27.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [metal 0.27.0](https://crates.io/crates/metal/0.27.0) |
| `metal` | `0.31.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [metal 0.31.0](https://crates.io/crates/metal/0.31.0) |
| `miniz_oxide` | `0.8.9` | `MIT OR Zlib OR Apache-2.0` | `LICENSE`, `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` | [miniz_oxide 0.8.9](https://crates.io/crates/miniz_oxide/0.8.9) |
| `moxcms` | `0.8.1` | `BSD-3-Clause OR Apache-2.0` | `LICENSE-APACHE.md`, `LICENSE.md` | [moxcms 0.8.1](https://crates.io/crates/moxcms/0.8.1) |
| `muda` | `0.17.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.spdx` | [muda 0.17.2](https://crates.io/crates/muda/0.17.2) |
| `naga` | `0.19.2` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [naga 0.19.2](https://crates.io/crates/naga/0.19.2) |
| `naga` | `25.0.1` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [naga 25.0.1](https://crates.io/crates/naga/25.0.1) |
| `native-tls` | `0.2.18` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [native-tls 0.2.18](https://crates.io/crates/native-tls/0.2.18) |
| `ndk` | `0.8.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ndk 0.8.0](https://crates.io/crates/ndk/0.8.0) |
| `ndk` | `0.9.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ndk 0.9.0](https://crates.io/crates/ndk/0.9.0) |
| `ndk-context` | `0.1.1` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ndk-context 0.1.1](https://crates.io/crates/ndk-context/0.1.1) |
| `ndk-sys` | `0.5.0+25.2.9519653` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ndk-sys 0.5.0+25.2.9519653](https://crates.io/crates/ndk-sys/0.5.0+25.2.9519653) |
| `ndk-sys` | `0.6.0+11769913` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [ndk-sys 0.6.0+11769913](https://crates.io/crates/ndk-sys/0.6.0+11769913) |
| `new_debug_unreachable` | `1.0.6` | `MIT` | `LICENSE-MIT` | [new_debug_unreachable 1.0.6](https://crates.io/crates/new_debug_unreachable/1.0.6) |
| `nix` | `0.30.1` | `MIT` | `LICENSE` | [nix 0.30.1](https://crates.io/crates/nix/0.30.1) |
| `nodrop` | `0.1.14` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [nodrop 0.1.14](https://crates.io/crates/nodrop/0.1.14) |
| `nohash-hasher` | `0.2.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [nohash-hasher 0.2.0](https://crates.io/crates/nohash-hasher/0.2.0) |
| `num-conv` | `0.2.1` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [num-conv 0.2.1](https://crates.io/crates/num-conv/0.2.1) |
| `num-derive` | `0.4.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [num-derive 0.4.2](https://crates.io/crates/num-derive/0.4.2) |
| `num-traits` | `0.2.19` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [num-traits 0.2.19](https://crates.io/crates/num-traits/0.2.19) |
| `num_enum` | `0.7.6` | `BSD-3-Clause OR MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` | [num_enum 0.7.6](https://crates.io/crates/num_enum/0.7.6) |
| `num_enum_derive` | `0.7.6` | `BSD-3-Clause OR MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` | [num_enum_derive 0.7.6](https://crates.io/crates/num_enum_derive/0.7.6) |
| `objc` | `0.2.7` | `MIT` | `LICENSE.txt` | [objc 0.2.7](https://crates.io/crates/objc/0.2.7) |
| `objc-sys` | `0.3.5` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc-sys 0.3.5](https://crates.io/crates/objc-sys/0.3.5) |
| `objc2` | `0.4.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2 0.4.1](https://crates.io/crates/objc2/0.4.1) |
| `objc2` | `0.5.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2 0.5.2](https://crates.io/crates/objc2/0.5.2) |
| `objc2` | `0.6.4` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2 0.6.4](https://crates.io/crates/objc2/0.6.4) |
| `objc2-app-kit` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-app-kit 0.2.2](https://crates.io/crates/objc2-app-kit/0.2.2) |
| `objc2-app-kit` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-app-kit 0.3.2](https://crates.io/crates/objc2-app-kit/0.3.2) |
| `objc2-audio-toolbox` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-audio-toolbox 0.3.2](https://crates.io/crates/objc2-audio-toolbox/0.3.2) |
| `objc2-cloud-kit` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-cloud-kit 0.2.2](https://crates.io/crates/objc2-cloud-kit/0.2.2) |
| `objc2-contacts` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-contacts 0.2.2](https://crates.io/crates/objc2-contacts/0.2.2) |
| `objc2-core-audio` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-audio 0.3.2](https://crates.io/crates/objc2-core-audio/0.3.2) |
| `objc2-core-audio-types` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-audio-types 0.3.2](https://crates.io/crates/objc2-core-audio-types/0.3.2) |
| `objc2-core-data` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-data 0.2.2](https://crates.io/crates/objc2-core-data/0.2.2) |
| `objc2-core-foundation` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-foundation 0.3.2](https://crates.io/crates/objc2-core-foundation/0.3.2) |
| `objc2-core-graphics` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-graphics 0.3.2](https://crates.io/crates/objc2-core-graphics/0.3.2) |
| `objc2-core-image` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-image 0.2.2](https://crates.io/crates/objc2-core-image/0.2.2) |
| `objc2-core-location` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-core-location 0.2.2](https://crates.io/crates/objc2-core-location/0.2.2) |
| `objc2-encode` | `3.0.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-encode 3.0.0](https://crates.io/crates/objc2-encode/3.0.0) |
| `objc2-encode` | `4.1.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-encode 4.1.0](https://crates.io/crates/objc2-encode/4.1.0) |
| `objc2-exception-helper` | `0.1.1` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-exception-helper 0.1.1](https://crates.io/crates/objc2-exception-helper/0.1.1) |
| `objc2-foundation` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-foundation 0.2.2](https://crates.io/crates/objc2-foundation/0.2.2) |
| `objc2-foundation` | `0.3.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-foundation 0.3.2](https://crates.io/crates/objc2-foundation/0.3.2) |
| `objc2-io-kit` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-io-kit 0.3.2](https://crates.io/crates/objc2-io-kit/0.3.2) |
| `objc2-io-surface` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-io-surface 0.3.2](https://crates.io/crates/objc2-io-surface/0.3.2) |
| `objc2-link-presentation` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-link-presentation 0.2.2](https://crates.io/crates/objc2-link-presentation/0.2.2) |
| `objc2-metal` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-metal 0.2.2](https://crates.io/crates/objc2-metal/0.2.2) |
| `objc2-quartz-core` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-quartz-core 0.2.2](https://crates.io/crates/objc2-quartz-core/0.2.2) |
| `objc2-symbols` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-symbols 0.2.2](https://crates.io/crates/objc2-symbols/0.2.2) |
| `objc2-ui-kit` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-ui-kit 0.2.2](https://crates.io/crates/objc2-ui-kit/0.2.2) |
| `objc2-ui-kit` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-ui-kit 0.3.2](https://crates.io/crates/objc2-ui-kit/0.3.2) |
| `objc2-uniform-type-identifiers` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-uniform-type-identifiers 0.2.2](https://crates.io/crates/objc2-uniform-type-identifiers/0.2.2) |
| `objc2-user-notifications` | `0.2.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-user-notifications 0.2.2](https://crates.io/crates/objc2-user-notifications/0.2.2) |
| `objc2-web-kit` | `0.3.2` | `Zlib OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [objc2-web-kit 0.3.2](https://crates.io/crates/objc2-web-kit/0.3.2) |
| `objc_exception` | `0.1.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc_exception 0.1.2](https://crates.io/crates/objc_exception/0.1.2) |
| `objc_id` | `0.1.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [objc_id 0.1.1](https://crates.io/crates/objc_id/0.1.1) |
| `once_cell` | `1.21.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [once_cell 1.21.4](https://crates.io/crates/once_cell/1.21.4) |
| `once_cell_polyfill` | `1.70.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [once_cell_polyfill 1.70.2](https://crates.io/crates/once_cell_polyfill/1.70.2) |
| `openssl` | `0.10.76` | `Apache-2.0` | `LICENSE`, `LICENSE-APACHE` | [openssl 0.10.76](https://crates.io/crates/openssl/0.10.76) |
| `openssl-macros` | `0.1.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [openssl-macros 0.1.1](https://crates.io/crates/openssl-macros/0.1.1) |
| `openssl-probe` | `0.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [openssl-probe 0.2.1](https://crates.io/crates/openssl-probe/0.2.1) |
| `openssl-sys` | `0.9.112` | `MIT` | `LICENSE-MIT` | [openssl-sys 0.9.112](https://crates.io/crates/openssl-sys/0.9.112) |
| `option-ext` | `0.2.0` | `MPL-2.0` | `LICENSE.txt` | [option-ext 0.2.0](https://crates.io/crates/option-ext/0.2.0) |
| `orbclient` | `0.3.51` | `MIT` | `LICENSE` | [orbclient 0.3.51](https://crates.io/crates/orbclient/0.3.51) |
| `ordered-float` | `4.6.0` | `MIT` | `LICENSE-MIT` | [ordered-float 4.6.0](https://crates.io/crates/ordered-float/4.6.0) |
| `ordered-stream` | `0.2.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ordered-stream 0.2.0](https://crates.io/crates/ordered-stream/0.2.0) |
| `owned_ttf_parser` | `0.25.1` | `Apache-2.0` | `LICENSE` | [owned_ttf_parser 0.25.1](https://crates.io/crates/owned_ttf_parser/0.25.1) |
| `pango` | `0.18.3` | `MIT` | `LICENSE` | [pango 0.18.3](https://crates.io/crates/pango/0.18.3) |
| `pango-sys` | `0.18.0` | `MIT` | `LICENSE` | [pango-sys 0.18.0](https://crates.io/crates/pango-sys/0.18.0) |
| `parking` | `2.2.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-THIRD-PARTY` | [parking 2.2.1](https://crates.io/crates/parking/2.2.1) |
| `parking_lot` | `0.12.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [parking_lot 0.12.5](https://crates.io/crates/parking_lot/0.12.5) |
| `parking_lot_core` | `0.9.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [parking_lot_core 0.9.12](https://crates.io/crates/parking_lot_core/0.9.12) |
| `password-hash` | `0.4.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [password-hash 0.4.2](https://crates.io/crates/password-hash/0.4.2) |
| `paste` | `1.0.15` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [paste 1.0.15](https://crates.io/crates/paste/1.0.15) |
| `pbkdf2` | `0.11.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pbkdf2 0.11.0](https://crates.io/crates/pbkdf2/0.11.0) |
| `percent-encoding` | `2.3.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [percent-encoding 2.3.2](https://crates.io/crates/percent-encoding/2.3.2) |
| `phf` | `0.10.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf 0.10.1](https://crates.io/crates/phf/0.10.1) |
| `phf` | `0.11.3` | `MIT` | `LICENSE` | [phf 0.11.3](https://crates.io/crates/phf/0.11.3) |
| `phf` | `0.8.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf 0.8.0](https://crates.io/crates/phf/0.8.0) |
| `phf_codegen` | `0.11.3` | `MIT` | `LICENSE` | [phf_codegen 0.11.3](https://crates.io/crates/phf_codegen/0.11.3) |
| `phf_codegen` | `0.8.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_codegen 0.8.0](https://crates.io/crates/phf_codegen/0.8.0) |
| `phf_generator` | `0.10.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_generator 0.10.0](https://crates.io/crates/phf_generator/0.10.0) |
| `phf_generator` | `0.11.3` | `MIT` | `LICENSE` | [phf_generator 0.11.3](https://crates.io/crates/phf_generator/0.11.3) |
| `phf_generator` | `0.8.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_generator 0.8.0](https://crates.io/crates/phf_generator/0.8.0) |
| `phf_macros` | `0.10.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_macros 0.10.0](https://crates.io/crates/phf_macros/0.10.0) |
| `phf_shared` | `0.10.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_shared 0.10.0](https://crates.io/crates/phf_shared/0.10.0) |
| `phf_shared` | `0.11.3` | `MIT` | `LICENSE` | [phf_shared 0.11.3](https://crates.io/crates/phf_shared/0.11.3) |
| `phf_shared` | `0.8.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [phf_shared 0.8.0](https://crates.io/crates/phf_shared/0.8.0) |
| `pin-project` | `1.1.11` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [pin-project 1.1.11](https://crates.io/crates/pin-project/1.1.11) |
| `pin-project-internal` | `1.1.11` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [pin-project-internal 1.1.11](https://crates.io/crates/pin-project-internal/1.1.11) |
| `pin-project-lite` | `0.2.17` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [pin-project-lite 0.2.17](https://crates.io/crates/pin-project-lite/0.2.17) |
| `piper` | `0.2.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [piper 0.2.5](https://crates.io/crates/piper/0.2.5) |
| `pixels` | `0.15.0` | `MIT` | `LICENSE` | [pixels 0.15.0](https://crates.io/crates/pixels/0.15.0) |
| `pkg-config` | `0.3.32` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pkg-config 0.3.32](https://crates.io/crates/pkg-config/0.3.32) |
| `plain` | `0.2.3` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [plain 0.2.3](https://crates.io/crates/plain/0.2.3) |
| `png` | `0.17.16` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [png 0.17.16](https://crates.io/crates/png/0.17.16) |
| `png` | `0.18.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [png 0.18.1](https://crates.io/crates/png/0.18.1) |
| `polling` | `3.11.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [polling 3.11.0](https://crates.io/crates/polling/3.11.0) |
| `pollster` | `0.3.0` | `Apache-2.0/MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [pollster 0.3.0](https://crates.io/crates/pollster/0.3.0) |
| `pollster` | `0.4.0` | `Apache-2.0/MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [pollster 0.4.0](https://crates.io/crates/pollster/0.4.0) |
| `portable-atomic` | `1.13.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [portable-atomic 1.13.1](https://crates.io/crates/portable-atomic/1.13.1) |
| `potential_utf` | `0.1.5` | `Unicode-3.0` | `LICENSE` | [potential_utf 0.1.5](https://crates.io/crates/potential_utf/0.1.5) |
| `powerfmt` | `0.2.0` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [powerfmt 0.2.0](https://crates.io/crates/powerfmt/0.2.0) |
| `ppv-lite86` | `0.2.21` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ppv-lite86 0.2.21](https://crates.io/crates/ppv-lite86/0.2.21) |
| `precomputed-hash` | `0.1.1` | `MIT` | `LICENSE` | [precomputed-hash 0.1.1](https://crates.io/crates/precomputed-hash/0.1.1) |
| `presser` | `0.3.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [presser 0.3.1](https://crates.io/crates/presser/0.3.1) |
| `proc-macro-crate` | `1.3.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-crate 1.3.1](https://crates.io/crates/proc-macro-crate/1.3.1) |
| `proc-macro-crate` | `2.0.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-crate 2.0.2](https://crates.io/crates/proc-macro-crate/2.0.2) |
| `proc-macro-crate` | `3.5.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-crate 3.5.0](https://crates.io/crates/proc-macro-crate/3.5.0) |
| `proc-macro-error` | `1.0.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-error 1.0.4](https://crates.io/crates/proc-macro-error/1.0.4) |
| `proc-macro-error-attr` | `1.0.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-error-attr 1.0.4](https://crates.io/crates/proc-macro-error-attr/1.0.4) |
| `proc-macro-hack` | `0.5.20+deprecated` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro-hack 0.5.20+deprecated](https://crates.io/crates/proc-macro-hack/0.5.20+deprecated) |
| `proc-macro2` | `1.0.106` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro2 1.0.106](https://crates.io/crates/proc-macro2/1.0.106) |
| `proc-macro2-diagnostics` | `0.10.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [proc-macro2-diagnostics 0.10.1](https://crates.io/crates/proc-macro2-diagnostics/0.10.1) |
| `profiling` | `1.0.17` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [profiling 1.0.17](https://crates.io/crates/profiling/1.0.17) |
| `pxfm` | `0.1.28` | `BSD-3-Clause OR Apache-2.0` | `LICENSE-APACHE.md`, `LICENSE.md` | [pxfm 0.1.28](https://crates.io/crates/pxfm/0.1.28) |
| `pyo3` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3 0.22.6](https://crates.io/crates/pyo3/0.22.6) |
| `pyo3-build-config` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3-build-config 0.22.6](https://crates.io/crates/pyo3-build-config/0.22.6) |
| `pyo3-ffi` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3-ffi 0.22.6](https://crates.io/crates/pyo3-ffi/0.22.6) |
| `pyo3-macros` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3-macros 0.22.6](https://crates.io/crates/pyo3-macros/0.22.6) |
| `pyo3-macros-backend` | `0.22.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [pyo3-macros-backend 0.22.6](https://crates.io/crates/pyo3-macros-backend/0.22.6) |
| `quick-error` | `2.0.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [quick-error 2.0.1](https://crates.io/crates/quick-error/2.0.1) |
| `quick-xml` | `0.38.4` | `MIT` | `LICENSE-MIT.md` | [quick-xml 0.38.4](https://crates.io/crates/quick-xml/0.38.4) |
| `quick-xml` | `0.39.2` | `MIT` | `LICENSE-MIT.md` | [quick-xml 0.39.2](https://crates.io/crates/quick-xml/0.39.2) |
| `quote` | `1.0.45` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [quote 1.0.45](https://crates.io/crates/quote/1.0.45) |
| `r-efi` | `5.3.0` | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | (no top-level license file; Cargo metadata expression only) | [r-efi 5.3.0](https://crates.io/crates/r-efi/5.3.0) |
| `rand` | `0.7.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand 0.7.3](https://crates.io/crates/rand/0.7.3) |
| `rand` | `0.8.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand 0.8.5](https://crates.io/crates/rand/0.8.5) |
| `rand` | `0.9.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand 0.9.2](https://crates.io/crates/rand/0.9.2) |
| `rand_chacha` | `0.2.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_chacha 0.2.2](https://crates.io/crates/rand_chacha/0.2.2) |
| `rand_chacha` | `0.3.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_chacha 0.3.1](https://crates.io/crates/rand_chacha/0.3.1) |
| `rand_chacha` | `0.9.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_chacha 0.9.0](https://crates.io/crates/rand_chacha/0.9.0) |
| `rand_core` | `0.5.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_core 0.5.1](https://crates.io/crates/rand_core/0.5.1) |
| `rand_core` | `0.6.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_core 0.6.4](https://crates.io/crates/rand_core/0.6.4) |
| `rand_core` | `0.9.5` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_core 0.9.5](https://crates.io/crates/rand_core/0.9.5) |
| `rand_hc` | `0.2.0` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_hc 0.2.0](https://crates.io/crates/rand_hc/0.2.0) |
| `rand_pcg` | `0.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rand_pcg 0.2.1](https://crates.io/crates/rand_pcg/0.2.1) |
| `range-alloc` | `0.1.5` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [range-alloc 0.1.5](https://crates.io/crates/range-alloc/0.1.5) |
| `raw-window-handle` | `0.5.2` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` | [raw-window-handle 0.5.2](https://crates.io/crates/raw-window-handle/0.5.2) |
| `raw-window-handle` | `0.6.2` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` | [raw-window-handle 0.6.2](https://crates.io/crates/raw-window-handle/0.6.2) |
| `redox_syscall` | `0.3.5` | `MIT` | `LICENSE` | [redox_syscall 0.3.5](https://crates.io/crates/redox_syscall/0.3.5) |
| `redox_syscall` | `0.4.1` | `MIT` | `LICENSE` | [redox_syscall 0.4.1](https://crates.io/crates/redox_syscall/0.4.1) |
| `redox_syscall` | `0.5.18` | `MIT` | `LICENSE` | [redox_syscall 0.5.18](https://crates.io/crates/redox_syscall/0.5.18) |
| `redox_syscall` | `0.7.3` | `MIT` | `LICENSE` | [redox_syscall 0.7.3](https://crates.io/crates/redox_syscall/0.7.3) |
| `redox_users` | `0.5.2` | `MIT` | `LICENSE` | [redox_users 0.5.2](https://crates.io/crates/redox_users/0.5.2) |
| `regex-automata` | `0.4.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [regex-automata 0.4.14](https://crates.io/crates/regex-automata/0.4.14) |
| `regex-syntax` | `0.8.10` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [regex-syntax 0.8.10](https://crates.io/crates/regex-syntax/0.8.10) |
| `renderdoc-sys` | `1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [renderdoc-sys 1.1.0](https://crates.io/crates/renderdoc-sys/1.1.0) |
| `rfd` | `0.17.2` | `MIT` | `LICENSE` | [rfd 0.17.2](https://crates.io/crates/rfd/0.17.2) |
| `ring` | `0.17.14` | `Apache-2.0 AND ISC` | `LICENSE`, `LICENSE-BoringSSL`, `LICENSE-other-bits` | [ring 0.17.14](https://crates.io/crates/ring/0.17.14) |
| `rustc-hash` | `1.1.0` | `Apache-2.0/MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [rustc-hash 1.1.0](https://crates.io/crates/rustc-hash/1.1.0) |
| `rustc-hash` | `2.1.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [rustc-hash 2.1.2](https://crates.io/crates/rustc-hash/2.1.2) |
| `rustc_version` | `0.4.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rustc_version 0.4.1](https://crates.io/crates/rustc_version/0.4.1) |
| `rustix` | `0.38.44` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [rustix 0.38.44](https://crates.io/crates/rustix/0.38.44) |
| `rustix` | `1.1.4` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [rustix 1.1.4](https://crates.io/crates/rustix/1.1.4) |
| `rustls` | `0.23.37` | `Apache-2.0 OR ISC OR MIT` | `LICENSE-APACHE`, `LICENSE-ISC`, `LICENSE-MIT` | [rustls 0.23.37](https://crates.io/crates/rustls/0.23.37) |
| `rustls-pki-types` | `1.14.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rustls-pki-types 1.14.0](https://crates.io/crates/rustls-pki-types/1.14.0) |
| `rustls-webpki` | `0.103.10` | `ISC` | `LICENSE` | [rustls-webpki 0.103.10](https://crates.io/crates/rustls-webpki/0.103.10) |
| `rustversion` | `1.0.22` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [rustversion 1.0.22](https://crates.io/crates/rustversion/1.0.22) |
| `safe_arch` | `0.7.4` | `Zlib OR Apache-2.0 OR MIT` | `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` | [safe_arch 0.7.4](https://crates.io/crates/safe_arch/0.7.4) |
| `same-file` | `1.0.6` | `Unlicense/MIT` | `COPYING`, `LICENSE-MIT` | [same-file 1.0.6](https://crates.io/crates/same-file/1.0.6) |
| `schannel` | `0.1.29` | `MIT` | `LICENSE.md` | [schannel 0.1.29](https://crates.io/crates/schannel/0.1.29) |
| `scoped-tls` | `1.0.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [scoped-tls 1.0.1](https://crates.io/crates/scoped-tls/1.0.1) |
| `scopeguard` | `1.2.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [scopeguard 1.2.0](https://crates.io/crates/scopeguard/1.2.0) |
| `sctk-adwaita` | `0.10.1` | `MIT` | `LICENSE` | [sctk-adwaita 0.10.1](https://crates.io/crates/sctk-adwaita/0.10.1) |
| `sctk-adwaita` | `0.8.3` | `MIT` | `LICENSE` | [sctk-adwaita 0.8.3](https://crates.io/crates/sctk-adwaita/0.8.3) |
| `sdl2` | `0.37.0` | `MIT` | `LICENSE` | [sdl2 0.37.0](https://crates.io/crates/sdl2/0.37.0) |
| `sdl2-sys` | `0.37.0` | `MIT AND Zlib` | (no top-level license file; Cargo metadata expression only) | [sdl2-sys 0.37.0](https://crates.io/crates/sdl2-sys/0.37.0) |
| `security-framework` | `3.7.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [security-framework 3.7.0](https://crates.io/crates/security-framework/3.7.0) |
| `security-framework-sys` | `2.17.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [security-framework-sys 2.17.0](https://crates.io/crates/security-framework-sys/2.17.0) |
| `selectors` | `0.24.0` | `MPL-2.0` | (no top-level license file; Cargo metadata expression only) | [selectors 0.24.0](https://crates.io/crates/selectors/0.24.0) |
| `semver` | `1.0.28` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [semver 1.0.28](https://crates.io/crates/semver/1.0.28) |
| `send_wrapper` | `0.6.0` | `MIT/Apache-2.0` | `LICENSE-APACHE.txt`, `LICENSE-MIT.txt` | [send_wrapper 0.6.0](https://crates.io/crates/send_wrapper/0.6.0) |
| `serde` | `1.0.228` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde 1.0.228](https://crates.io/crates/serde/1.0.228) |
| `serde-wasm-bindgen` | `0.6.5` | `MIT` | `LICENSE` | [serde-wasm-bindgen 0.6.5](https://crates.io/crates/serde-wasm-bindgen/0.6.5) |
| `serde_core` | `1.0.228` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_core 1.0.228](https://crates.io/crates/serde_core/1.0.228) |
| `serde_derive` | `1.0.228` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_derive 1.0.228](https://crates.io/crates/serde_derive/1.0.228) |
| `serde_json` | `1.0.149` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_json 1.0.149](https://crates.io/crates/serde_json/1.0.149) |
| `serde_repr` | `0.1.20` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_repr 0.1.20](https://crates.io/crates/serde_repr/0.1.20) |
| `serde_spanned` | `0.6.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [serde_spanned 0.6.9](https://crates.io/crates/serde_spanned/0.6.9) |
| `servo_arc` | `0.2.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [servo_arc 0.2.0](https://crates.io/crates/servo_arc/0.2.0) |
| `sha1` | `0.10.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [sha1 0.10.6](https://crates.io/crates/sha1/0.10.6) |
| `sha2` | `0.10.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [sha2 0.10.9](https://crates.io/crates/sha2/0.10.9) |
| `sharded-slab` | `0.1.7` | `MIT` | `LICENSE` | [sharded-slab 0.1.7](https://crates.io/crates/sharded-slab/0.1.7) |
| `shlex` | `1.3.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [shlex 1.3.0](https://crates.io/crates/shlex/1.3.0) |
| `signal-hook` | `0.3.18` | `Apache-2.0/MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [signal-hook 0.3.18](https://crates.io/crates/signal-hook/0.3.18) |
| `signal-hook-registry` | `1.4.8` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [signal-hook-registry 1.4.8](https://crates.io/crates/signal-hook-registry/1.4.8) |
| `simd-adler32` | `0.3.9` | `MIT` | `LICENSE.md` | [simd-adler32 0.3.9](https://crates.io/crates/simd-adler32/0.3.9) |
| `simd_cesu8` | `1.1.1` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [simd_cesu8 1.1.1](https://crates.io/crates/simd_cesu8/1.1.1) |
| `simdutf8` | `0.1.5` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [simdutf8 0.1.5](https://crates.io/crates/simdutf8/0.1.5) |
| `siphasher` | `0.3.11` | `MIT/Apache-2.0` | `COPYING` | [siphasher 0.3.11](https://crates.io/crates/siphasher/0.3.11) |
| `siphasher` | `1.0.2` | `MIT/Apache-2.0` | `COPYING` | [siphasher 1.0.2](https://crates.io/crates/siphasher/1.0.2) |
| `slab` | `0.4.12` | `MIT` | `LICENSE` | [slab 0.4.12](https://crates.io/crates/slab/0.4.12) |
| `sledgehammer_bindgen` | `0.6.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [sledgehammer_bindgen 0.6.0](https://crates.io/crates/sledgehammer_bindgen/0.6.0) |
| `sledgehammer_bindgen_macro` | `0.6.5` | `MIT` | (no top-level license file; Cargo metadata expression only) | [sledgehammer_bindgen_macro 0.6.5](https://crates.io/crates/sledgehammer_bindgen_macro/0.6.5) |
| `sledgehammer_utils` | `0.3.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [sledgehammer_utils 0.3.1](https://crates.io/crates/sledgehammer_utils/0.3.1) |
| `slotmap` | `1.1.1` | `Zlib` | `LICENSE` | [slotmap 1.1.1](https://crates.io/crates/slotmap/1.1.1) |
| `smallvec` | `1.15.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [smallvec 1.15.1](https://crates.io/crates/smallvec/1.15.1) |
| `smithay-client-toolkit` | `0.18.1` | `MIT` | `LICENSE.txt` | [smithay-client-toolkit 0.18.1](https://crates.io/crates/smithay-client-toolkit/0.18.1) |
| `smithay-client-toolkit` | `0.19.2` | `MIT` | `LICENSE.txt` | [smithay-client-toolkit 0.19.2](https://crates.io/crates/smithay-client-toolkit/0.19.2) |
| `smithay-client-toolkit` | `0.20.0` | `MIT` | `LICENSE.txt` | [smithay-client-toolkit 0.20.0](https://crates.io/crates/smithay-client-toolkit/0.20.0) |
| `smithay-clipboard` | `0.7.3` | `MIT` | `LICENSE` | [smithay-clipboard 0.7.3](https://crates.io/crates/smithay-clipboard/0.7.3) |
| `smol_str` | `0.2.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [smol_str 0.2.2](https://crates.io/crates/smol_str/0.2.2) |
| `soup3` | `0.5.0` | `MIT` | `LICENSE` | [soup3 0.5.0](https://crates.io/crates/soup3/0.5.0) |
| `soup3-sys` | `0.5.0` | `MIT` | `LICENSE` | [soup3-sys 0.5.0](https://crates.io/crates/soup3-sys/0.5.0) |
| `spirv` | `0.3.0+sdk-1.3.268.0` | `Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [spirv 0.3.0+sdk-1.3.268.0](https://crates.io/crates/spirv/0.3.0+sdk-1.3.268.0) |
| `stable_deref_trait` | `1.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [stable_deref_trait 1.2.1](https://crates.io/crates/stable_deref_trait/1.2.1) |
| `static_assertions` | `1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [static_assertions 1.1.0](https://crates.io/crates/static_assertions/1.1.0) |
| `strict-num` | `0.1.1` | `MIT` | `LICENSE` | [strict-num 0.1.1](https://crates.io/crates/strict-num/0.1.1) |
| `string_cache` | `0.8.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [string_cache 0.8.9](https://crates.io/crates/string_cache/0.8.9) |
| `string_cache_codegen` | `0.5.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [string_cache_codegen 0.5.4](https://crates.io/crates/string_cache_codegen/0.5.4) |
| `strsim` | `0.11.1` | `MIT` | `LICENSE` | [strsim 0.11.1](https://crates.io/crates/strsim/0.11.1) |
| `strum` | `0.26.3` | `MIT` | `LICENSE` | [strum 0.26.3](https://crates.io/crates/strum/0.26.3) |
| `strum_macros` | `0.26.4` | `MIT` | `LICENSE` | [strum_macros 0.26.4](https://crates.io/crates/strum_macros/0.26.4) |
| `subsecond` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [subsecond 0.7.4](https://crates.io/crates/subsecond/0.7.4) |
| `subsecond-types` | `0.7.4` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [subsecond-types 0.7.4](https://crates.io/crates/subsecond-types/0.7.4) |
| `subtle` | `2.6.1` | `BSD-3-Clause` | `LICENSE` | [subtle 2.6.1](https://crates.io/crates/subtle/2.6.1) |
| `syn` | `1.0.109` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [syn 1.0.109](https://crates.io/crates/syn/1.0.109) |
| `syn` | `2.0.117` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [syn 2.0.117](https://crates.io/crates/syn/2.0.117) |
| `synstructure` | `0.13.2` | `MIT` | `LICENSE` | [synstructure 0.13.2](https://crates.io/crates/synstructure/0.13.2) |
| `system-deps` | `6.2.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [system-deps 6.2.2](https://crates.io/crates/system-deps/6.2.2) |
| `tao` | `0.34.8` | `Apache-2.0` | `LICENSE`, `LICENSE.spdx` | [tao 0.34.8](https://crates.io/crates/tao/0.34.8) |
| `tao-macros` | `0.1.3` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [tao-macros 0.1.3](https://crates.io/crates/tao-macros/0.1.3) |
| `target-lexicon` | `0.12.16` | `Apache-2.0 WITH LLVM-exception` | `LICENSE` | [target-lexicon 0.12.16](https://crates.io/crates/target-lexicon/0.12.16) |
| `tempfile` | `3.27.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [tempfile 3.27.0](https://crates.io/crates/tempfile/3.27.0) |
| `tendril` | `0.4.3` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [tendril 0.4.3](https://crates.io/crates/tendril/0.4.3) |
| `termcolor` | `1.4.1` | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT` | [termcolor 1.4.1](https://crates.io/crates/termcolor/1.4.1) |
| `thiserror` | `1.0.69` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thiserror 1.0.69](https://crates.io/crates/thiserror/1.0.69) |
| `thiserror` | `2.0.18` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thiserror 2.0.18](https://crates.io/crates/thiserror/2.0.18) |
| `thiserror-impl` | `1.0.69` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thiserror-impl 1.0.69](https://crates.io/crates/thiserror-impl/1.0.69) |
| `thiserror-impl` | `2.0.18` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thiserror-impl 2.0.18](https://crates.io/crates/thiserror-impl/2.0.18) |
| `thread_local` | `1.1.9` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [thread_local 1.1.9](https://crates.io/crates/thread_local/1.1.9) |
| `tiff` | `0.11.3` | `MIT` | `LICENSE` | [tiff 0.11.3](https://crates.io/crates/tiff/0.11.3) |
| `time` | `0.3.47` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [time 0.3.47](https://crates.io/crates/time/0.3.47) |
| `time-core` | `0.1.8` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [time-core 0.1.8](https://crates.io/crates/time-core/0.1.8) |
| `time-macros` | `0.2.27` | `MIT OR Apache-2.0` | `LICENSE-Apache`, `LICENSE-MIT` | [time-macros 0.2.27](https://crates.io/crates/time-macros/0.2.27) |
| `tiny-skia` | `0.11.4` | `BSD-3-Clause` | `LICENSE` | [tiny-skia 0.11.4](https://crates.io/crates/tiny-skia/0.11.4) |
| `tiny-skia-path` | `0.11.4` | `BSD-3-Clause` | `LICENSE` | [tiny-skia-path 0.11.4](https://crates.io/crates/tiny-skia-path/0.11.4) |
| `tinystr` | `0.8.3` | `Unicode-3.0` | `LICENSE` | [tinystr 0.8.3](https://crates.io/crates/tinystr/0.8.3) |
| `tokio` | `1.51.1` | `MIT` | `LICENSE` | [tokio 1.51.1](https://crates.io/crates/tokio/1.51.1) |
| `tokio-macros` | `2.7.0` | `MIT` | `LICENSE` | [tokio-macros 2.7.0](https://crates.io/crates/tokio-macros/2.7.0) |
| `toml` | `0.8.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml 0.8.2](https://crates.io/crates/toml/0.8.2) |
| `toml_datetime` | `0.6.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_datetime 0.6.3](https://crates.io/crates/toml_datetime/0.6.3) |
| `toml_datetime` | `1.1.1+spec-1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_datetime 1.1.1+spec-1.1.0](https://crates.io/crates/toml_datetime/1.1.1+spec-1.1.0) |
| `toml_edit` | `0.19.15` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_edit 0.19.15](https://crates.io/crates/toml_edit/0.19.15) |
| `toml_edit` | `0.20.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_edit 0.20.2](https://crates.io/crates/toml_edit/0.20.2) |
| `toml_edit` | `0.25.10+spec-1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_edit 0.25.10+spec-1.1.0](https://crates.io/crates/toml_edit/0.25.10+spec-1.1.0) |
| `toml_parser` | `1.1.2+spec-1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [toml_parser 1.1.2+spec-1.1.0](https://crates.io/crates/toml_parser/1.1.2+spec-1.1.0) |
| `tracing` | `0.1.44` | `MIT` | `LICENSE` | [tracing 0.1.44](https://crates.io/crates/tracing/0.1.44) |
| `tracing-attributes` | `0.1.31` | `MIT` | `LICENSE` | [tracing-attributes 0.1.31](https://crates.io/crates/tracing-attributes/0.1.31) |
| `tracing-core` | `0.1.36` | `MIT` | `LICENSE` | [tracing-core 0.1.36](https://crates.io/crates/tracing-core/0.1.36) |
| `tracing-subscriber` | `0.3.23` | `MIT` | `LICENSE` | [tracing-subscriber 0.3.23](https://crates.io/crates/tracing-subscriber/0.3.23) |
| `tracing-wasm` | `0.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [tracing-wasm 0.2.1](https://crates.io/crates/tracing-wasm/0.2.1) |
| `tray-icon` | `0.21.3` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.spdx` | [tray-icon 0.21.3](https://crates.io/crates/tray-icon/0.21.3) |
| `ttf-parser` | `0.21.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ttf-parser 0.21.1](https://crates.io/crates/ttf-parser/0.21.1) |
| `ttf-parser` | `0.25.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [ttf-parser 0.25.1](https://crates.io/crates/ttf-parser/0.25.1) |
| `tungstenite` | `0.28.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [tungstenite 0.28.0](https://crates.io/crates/tungstenite/0.28.0) |
| `type-map` | `0.5.1` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [type-map 0.5.1](https://crates.io/crates/type-map/0.5.1) |
| `typenum` | `1.19.0` | `MIT OR Apache-2.0` | `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT` | [typenum 1.19.0](https://crates.io/crates/typenum/1.19.0) |
| `uds_windows` | `1.2.1` | `MIT` | `LICENSE` | [uds_windows 1.2.1](https://crates.io/crates/uds_windows/1.2.1) |
| `ultraviolet` | `0.9.2` | `MIT OR Apache-2.0 OR Zlib` | (no top-level license file; Cargo metadata expression only) | [ultraviolet 0.9.2](https://crates.io/crates/ultraviolet/0.9.2) |
| `unicode-ident` | `1.0.24` | `(MIT OR Apache-2.0) AND Unicode-3.0` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-UNICODE` | [unicode-ident 1.0.24](https://crates.io/crates/unicode-ident/1.0.24) |
| `unicode-segmentation` | `1.13.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [unicode-segmentation 1.13.2](https://crates.io/crates/unicode-segmentation/1.13.2) |
| `unicode-width` | `0.1.14` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [unicode-width 0.1.14](https://crates.io/crates/unicode-width/0.1.14) |
| `unicode-xid` | `0.2.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [unicode-xid 0.2.6](https://crates.io/crates/unicode-xid/0.2.6) |
| `unindent` | `0.2.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [unindent 0.2.4](https://crates.io/crates/unindent/0.2.4) |
| `untrusted` | `0.9.0` | `ISC` | `LICENSE.txt` | [untrusted 0.9.0](https://crates.io/crates/untrusted/0.9.0) |
| `url` | `2.5.8` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [url 2.5.8](https://crates.io/crates/url/2.5.8) |
| `utf-8` | `0.7.6` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [utf-8 0.7.6](https://crates.io/crates/utf-8/0.7.6) |
| `utf8_iter` | `1.0.4` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [utf8_iter 1.0.4](https://crates.io/crates/utf8_iter/1.0.4) |
| `utf8parse` | `0.2.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [utf8parse 0.2.2](https://crates.io/crates/utf8parse/0.2.2) |
| `uuid` | `1.23.0` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [uuid 1.23.0](https://crates.io/crates/uuid/1.23.0) |
| `vcpkg` | `0.2.15` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [vcpkg 0.2.15](https://crates.io/crates/vcpkg/0.2.15) |
| `vec_map` | `0.8.2` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [vec_map 0.8.2](https://crates.io/crates/vec_map/0.8.2) |
| `version-compare` | `0.1.1` | `MIT` | `LICENSE` | [version-compare 0.1.1](https://crates.io/crates/version-compare/0.1.1) |
| `version-compare` | `0.2.1` | `MIT` | `LICENSE` | [version-compare 0.2.1](https://crates.io/crates/version-compare/0.2.1) |
| `version_check` | `0.9.5` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [version_check 0.9.5](https://crates.io/crates/version_check/0.9.5) |
| `walkdir` | `2.5.0` | `Unlicense/MIT` | `COPYING`, `LICENSE-MIT` | [walkdir 2.5.0](https://crates.io/crates/walkdir/2.5.0) |
| `warnings` | `0.2.1` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [warnings 0.2.1](https://crates.io/crates/warnings/0.2.1) |
| `warnings-macro` | `0.2.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [warnings-macro 0.2.0](https://crates.io/crates/warnings-macro/0.2.0) |
| `wasi` | `0.11.1+wasi-snapshot-preview1` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [wasi 0.11.1+wasi-snapshot-preview1](https://crates.io/crates/wasi/0.11.1+wasi-snapshot-preview1) |
| `wasi` | `0.9.0+wasi-snapshot-preview1` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [wasi 0.9.0+wasi-snapshot-preview1](https://crates.io/crates/wasi/0.9.0+wasi-snapshot-preview1) |
| `wasip2` | `1.0.2+wasi-0.2.9` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | (no top-level license file; Cargo metadata expression only) | [wasip2 1.0.2+wasi-0.2.9](https://crates.io/crates/wasip2/1.0.2+wasi-0.2.9) |
| `wasm-bindgen` | `0.2.117` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-bindgen 0.2.117](https://crates.io/crates/wasm-bindgen/0.2.117) |
| `wasm-bindgen-futures` | `0.4.67` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-bindgen-futures 0.4.67](https://crates.io/crates/wasm-bindgen-futures/0.4.67) |
| `wasm-bindgen-macro` | `0.2.117` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-bindgen-macro 0.2.117](https://crates.io/crates/wasm-bindgen-macro/0.2.117) |
| `wasm-bindgen-macro-support` | `0.2.117` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-bindgen-macro-support 0.2.117](https://crates.io/crates/wasm-bindgen-macro-support/0.2.117) |
| `wasm-bindgen-shared` | `0.2.117` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-bindgen-shared 0.2.117](https://crates.io/crates/wasm-bindgen-shared/0.2.117) |
| `wasm-streams` | `0.4.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [wasm-streams 0.4.2](https://crates.io/crates/wasm-streams/0.4.2) |
| `wayland-backend` | `0.3.15` | `MIT` | `LICENSE.txt` | [wayland-backend 0.3.15](https://crates.io/crates/wayland-backend/0.3.15) |
| `wayland-client` | `0.31.14` | `MIT` | `LICENSE.txt` | [wayland-client 0.31.14](https://crates.io/crates/wayland-client/0.31.14) |
| `wayland-csd-frame` | `0.3.0` | `MIT` | `LICENSE` | [wayland-csd-frame 0.3.0](https://crates.io/crates/wayland-csd-frame/0.3.0) |
| `wayland-cursor` | `0.31.14` | `MIT` | `LICENSE.txt` | [wayland-cursor 0.31.14](https://crates.io/crates/wayland-cursor/0.31.14) |
| `wayland-protocols` | `0.31.2` | `MIT` | `LICENSE.txt` | [wayland-protocols 0.31.2](https://crates.io/crates/wayland-protocols/0.31.2) |
| `wayland-protocols` | `0.32.12` | `MIT` | `LICENSE.txt` | [wayland-protocols 0.32.12](https://crates.io/crates/wayland-protocols/0.32.12) |
| `wayland-protocols-experimental` | `20250721.0.1` | `MIT` | `LICENSE.txt` | [wayland-protocols-experimental 20250721.0.1](https://crates.io/crates/wayland-protocols-experimental/20250721.0.1) |
| `wayland-protocols-misc` | `0.3.12` | `MIT` | `LICENSE.txt` | [wayland-protocols-misc 0.3.12](https://crates.io/crates/wayland-protocols-misc/0.3.12) |
| `wayland-protocols-plasma` | `0.2.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [wayland-protocols-plasma 0.2.0](https://crates.io/crates/wayland-protocols-plasma/0.2.0) |
| `wayland-protocols-plasma` | `0.3.12` | `MIT` | `LICENSE.txt` | [wayland-protocols-plasma 0.3.12](https://crates.io/crates/wayland-protocols-plasma/0.3.12) |
| `wayland-protocols-wlr` | `0.2.0` | `MIT` | (no top-level license file; Cargo metadata expression only) | [wayland-protocols-wlr 0.2.0](https://crates.io/crates/wayland-protocols-wlr/0.2.0) |
| `wayland-protocols-wlr` | `0.3.12` | `MIT` | `LICENSE.txt` | [wayland-protocols-wlr 0.3.12](https://crates.io/crates/wayland-protocols-wlr/0.3.12) |
| `wayland-scanner` | `0.31.10` | `MIT` | `LICENSE.txt` | [wayland-scanner 0.31.10](https://crates.io/crates/wayland-scanner/0.31.10) |
| `wayland-sys` | `0.31.11` | `MIT` | `LICENSE.txt` | [wayland-sys 0.31.11](https://crates.io/crates/wayland-sys/0.31.11) |
| `web-sys` | `0.3.94` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [web-sys 0.3.94](https://crates.io/crates/web-sys/0.3.94) |
| `web-time` | `0.2.4` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [web-time 0.2.4](https://crates.io/crates/web-time/0.2.4) |
| `web-time` | `1.1.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [web-time 1.1.0](https://crates.io/crates/web-time/1.1.0) |
| `webbrowser` | `1.2.0` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [webbrowser 1.2.0](https://crates.io/crates/webbrowser/1.2.0) |
| `webkit2gtk` | `2.0.1` | `MIT` | `LICENSE` | [webkit2gtk 2.0.1](https://crates.io/crates/webkit2gtk/2.0.1) |
| `webkit2gtk-sys` | `2.0.1` | `MIT` | `LICENSE` | [webkit2gtk-sys 2.0.1](https://crates.io/crates/webkit2gtk-sys/2.0.1) |
| `webview2-com` | `0.38.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [webview2-com 0.38.2](https://crates.io/crates/webview2-com/0.38.2) |
| `webview2-com-macros` | `0.8.1` | `MIT` | (no top-level license file; Cargo metadata expression only) | [webview2-com-macros 0.8.1](https://crates.io/crates/webview2-com-macros/0.8.1) |
| `webview2-com-sys` | `0.38.2` | `MIT` | (no top-level license file; Cargo metadata expression only) | [webview2-com-sys 0.38.2](https://crates.io/crates/webview2-com-sys/0.38.2) |
| `weezl` | `0.1.12` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [weezl 0.1.12](https://crates.io/crates/weezl/0.1.12) |
| `wgpu` | `0.19.4` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu 0.19.4](https://crates.io/crates/wgpu/0.19.4) |
| `wgpu` | `25.0.2` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu 25.0.2](https://crates.io/crates/wgpu/25.0.2) |
| `wgpu-core` | `0.19.4` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-core 0.19.4](https://crates.io/crates/wgpu-core/0.19.4) |
| `wgpu-core` | `25.0.2` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-core 25.0.2](https://crates.io/crates/wgpu-core/25.0.2) |
| `wgpu-core-deps-apple` | `25.0.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [wgpu-core-deps-apple 25.0.0](https://crates.io/crates/wgpu-core-deps-apple/25.0.0) |
| `wgpu-core-deps-emscripten` | `25.0.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [wgpu-core-deps-emscripten 25.0.0](https://crates.io/crates/wgpu-core-deps-emscripten/25.0.0) |
| `wgpu-core-deps-windows-linux-android` | `25.0.0` | `MIT OR Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [wgpu-core-deps-windows-linux-android 25.0.0](https://crates.io/crates/wgpu-core-deps-windows-linux-android/25.0.0) |
| `wgpu-hal` | `0.19.5` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-hal 0.19.5](https://crates.io/crates/wgpu-hal/0.19.5) |
| `wgpu-hal` | `25.0.2` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-hal 25.0.2](https://crates.io/crates/wgpu-hal/25.0.2) |
| `wgpu-types` | `0.19.2` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-types 0.19.2](https://crates.io/crates/wgpu-types/0.19.2) |
| `wgpu-types` | `25.0.0` | `MIT OR Apache-2.0` | `LICENSE.APACHE`, `LICENSE.MIT` | [wgpu-types 25.0.0](https://crates.io/crates/wgpu-types/25.0.0) |
| `wide` | `0.7.33` | `Zlib OR Apache-2.0 OR MIT` | `LICENSE-ZLIB.md` | [wide 0.7.33](https://crates.io/crates/wide/0.7.33) |
| `widestring` | `1.2.1` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [widestring 1.2.1](https://crates.io/crates/widestring/1.2.1) |
| `winapi` | `0.3.9` | `MIT/Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [winapi 0.3.9](https://crates.io/crates/winapi/0.3.9) |
| `winapi-i686-pc-windows-gnu` | `0.4.0` | `MIT/Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [winapi-i686-pc-windows-gnu 0.4.0](https://crates.io/crates/winapi-i686-pc-windows-gnu/0.4.0) |
| `winapi-util` | `0.1.11` | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT` | [winapi-util 0.1.11](https://crates.io/crates/winapi-util/0.1.11) |
| `winapi-x86_64-pc-windows-gnu` | `0.4.0` | `MIT/Apache-2.0` | (no top-level license file; Cargo metadata expression only) | [winapi-x86_64-pc-windows-gnu 0.4.0](https://crates.io/crates/winapi-x86_64-pc-windows-gnu/0.4.0) |
| `windows` | `0.52.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows 0.52.0](https://crates.io/crates/windows/0.52.0) |
| `windows` | `0.54.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows 0.54.0](https://crates.io/crates/windows/0.54.0) |
| `windows` | `0.58.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows 0.58.0](https://crates.io/crates/windows/0.58.0) |
| `windows` | `0.61.3` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows 0.61.3](https://crates.io/crates/windows/0.61.3) |
| `windows-collections` | `0.2.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-collections 0.2.0](https://crates.io/crates/windows-collections/0.2.0) |
| `windows-core` | `0.52.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-core 0.52.0](https://crates.io/crates/windows-core/0.52.0) |
| `windows-core` | `0.54.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-core 0.54.0](https://crates.io/crates/windows-core/0.54.0) |
| `windows-core` | `0.58.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-core 0.58.0](https://crates.io/crates/windows-core/0.58.0) |
| `windows-core` | `0.61.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-core 0.61.2](https://crates.io/crates/windows-core/0.61.2) |
| `windows-future` | `0.2.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-future 0.2.1](https://crates.io/crates/windows-future/0.2.1) |
| `windows-implement` | `0.58.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-implement 0.58.0](https://crates.io/crates/windows-implement/0.58.0) |
| `windows-implement` | `0.60.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-implement 0.60.2](https://crates.io/crates/windows-implement/0.60.2) |
| `windows-interface` | `0.58.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-interface 0.58.0](https://crates.io/crates/windows-interface/0.58.0) |
| `windows-interface` | `0.59.3` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-interface 0.59.3](https://crates.io/crates/windows-interface/0.59.3) |
| `windows-link` | `0.1.3` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-link 0.1.3](https://crates.io/crates/windows-link/0.1.3) |
| `windows-link` | `0.2.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-link 0.2.1](https://crates.io/crates/windows-link/0.2.1) |
| `windows-numerics` | `0.2.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-numerics 0.2.0](https://crates.io/crates/windows-numerics/0.2.0) |
| `windows-result` | `0.1.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-result 0.1.2](https://crates.io/crates/windows-result/0.1.2) |
| `windows-result` | `0.2.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-result 0.2.0](https://crates.io/crates/windows-result/0.2.0) |
| `windows-result` | `0.3.4` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-result 0.3.4](https://crates.io/crates/windows-result/0.3.4) |
| `windows-strings` | `0.1.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-strings 0.1.0](https://crates.io/crates/windows-strings/0.1.0) |
| `windows-strings` | `0.4.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-strings 0.4.2](https://crates.io/crates/windows-strings/0.4.2) |
| `windows-sys` | `0.45.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.45.0](https://crates.io/crates/windows-sys/0.45.0) |
| `windows-sys` | `0.48.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.48.0](https://crates.io/crates/windows-sys/0.48.0) |
| `windows-sys` | `0.52.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.52.0](https://crates.io/crates/windows-sys/0.52.0) |
| `windows-sys` | `0.59.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.59.0](https://crates.io/crates/windows-sys/0.59.0) |
| `windows-sys` | `0.60.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.60.2](https://crates.io/crates/windows-sys/0.60.2) |
| `windows-sys` | `0.61.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-sys 0.61.2](https://crates.io/crates/windows-sys/0.61.2) |
| `windows-targets` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-targets 0.42.2](https://crates.io/crates/windows-targets/0.42.2) |
| `windows-targets` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-targets 0.48.5](https://crates.io/crates/windows-targets/0.48.5) |
| `windows-targets` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-targets 0.52.6](https://crates.io/crates/windows-targets/0.52.6) |
| `windows-targets` | `0.53.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-targets 0.53.5](https://crates.io/crates/windows-targets/0.53.5) |
| `windows-threading` | `0.1.0` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-threading 0.1.0](https://crates.io/crates/windows-threading/0.1.0) |
| `windows-version` | `0.1.7` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows-version 0.1.7](https://crates.io/crates/windows-version/0.1.7) |
| `windows_aarch64_gnullvm` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_gnullvm 0.42.2](https://crates.io/crates/windows_aarch64_gnullvm/0.42.2) |
| `windows_aarch64_gnullvm` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_gnullvm 0.48.5](https://crates.io/crates/windows_aarch64_gnullvm/0.48.5) |
| `windows_aarch64_gnullvm` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_gnullvm 0.52.6](https://crates.io/crates/windows_aarch64_gnullvm/0.52.6) |
| `windows_aarch64_gnullvm` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_gnullvm 0.53.1](https://crates.io/crates/windows_aarch64_gnullvm/0.53.1) |
| `windows_aarch64_msvc` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_msvc 0.42.2](https://crates.io/crates/windows_aarch64_msvc/0.42.2) |
| `windows_aarch64_msvc` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_msvc 0.48.5](https://crates.io/crates/windows_aarch64_msvc/0.48.5) |
| `windows_aarch64_msvc` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_msvc 0.52.6](https://crates.io/crates/windows_aarch64_msvc/0.52.6) |
| `windows_aarch64_msvc` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_aarch64_msvc 0.53.1](https://crates.io/crates/windows_aarch64_msvc/0.53.1) |
| `windows_i686_gnu` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnu 0.42.2](https://crates.io/crates/windows_i686_gnu/0.42.2) |
| `windows_i686_gnu` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnu 0.48.5](https://crates.io/crates/windows_i686_gnu/0.48.5) |
| `windows_i686_gnu` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnu 0.52.6](https://crates.io/crates/windows_i686_gnu/0.52.6) |
| `windows_i686_gnu` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnu 0.53.1](https://crates.io/crates/windows_i686_gnu/0.53.1) |
| `windows_i686_gnullvm` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnullvm 0.52.6](https://crates.io/crates/windows_i686_gnullvm/0.52.6) |
| `windows_i686_gnullvm` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_gnullvm 0.53.1](https://crates.io/crates/windows_i686_gnullvm/0.53.1) |
| `windows_i686_msvc` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_msvc 0.42.2](https://crates.io/crates/windows_i686_msvc/0.42.2) |
| `windows_i686_msvc` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_msvc 0.48.5](https://crates.io/crates/windows_i686_msvc/0.48.5) |
| `windows_i686_msvc` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_msvc 0.52.6](https://crates.io/crates/windows_i686_msvc/0.52.6) |
| `windows_i686_msvc` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_i686_msvc 0.53.1](https://crates.io/crates/windows_i686_msvc/0.53.1) |
| `windows_x86_64_gnu` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnu 0.42.2](https://crates.io/crates/windows_x86_64_gnu/0.42.2) |
| `windows_x86_64_gnu` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnu 0.48.5](https://crates.io/crates/windows_x86_64_gnu/0.48.5) |
| `windows_x86_64_gnu` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnu 0.52.6](https://crates.io/crates/windows_x86_64_gnu/0.52.6) |
| `windows_x86_64_gnu` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnu 0.53.1](https://crates.io/crates/windows_x86_64_gnu/0.53.1) |
| `windows_x86_64_gnullvm` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnullvm 0.42.2](https://crates.io/crates/windows_x86_64_gnullvm/0.42.2) |
| `windows_x86_64_gnullvm` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnullvm 0.48.5](https://crates.io/crates/windows_x86_64_gnullvm/0.48.5) |
| `windows_x86_64_gnullvm` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnullvm 0.52.6](https://crates.io/crates/windows_x86_64_gnullvm/0.52.6) |
| `windows_x86_64_gnullvm` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_gnullvm 0.53.1](https://crates.io/crates/windows_x86_64_gnullvm/0.53.1) |
| `windows_x86_64_msvc` | `0.42.2` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_msvc 0.42.2](https://crates.io/crates/windows_x86_64_msvc/0.42.2) |
| `windows_x86_64_msvc` | `0.48.5` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_msvc 0.48.5](https://crates.io/crates/windows_x86_64_msvc/0.48.5) |
| `windows_x86_64_msvc` | `0.52.6` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_msvc 0.52.6](https://crates.io/crates/windows_x86_64_msvc/0.52.6) |
| `windows_x86_64_msvc` | `0.53.1` | `MIT OR Apache-2.0` | `license-apache-2.0`, `license-mit` | [windows_x86_64_msvc 0.53.1](https://crates.io/crates/windows_x86_64_msvc/0.53.1) |
| `winit` | `0.29.15` | `Apache-2.0` | `LICENSE` | [winit 0.29.15](https://crates.io/crates/winit/0.29.15) |
| `winit` | `0.30.13` | `Apache-2.0` | `LICENSE` | [winit 0.30.13](https://crates.io/crates/winit/0.30.13) |
| `winnow` | `0.5.40` | `MIT` | `LICENSE-MIT` | [winnow 0.5.40](https://crates.io/crates/winnow/0.5.40) |
| `winnow` | `0.7.15` | `MIT` | `LICENSE-MIT` | [winnow 0.7.15](https://crates.io/crates/winnow/0.7.15) |
| `winnow` | `1.0.1` | `MIT` | `LICENSE-MIT` | [winnow 1.0.1](https://crates.io/crates/winnow/1.0.1) |
| `wit-bindgen` | `0.51.0` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-Apache-2.0_WITH_LLVM-exception`, `LICENSE-MIT` | [wit-bindgen 0.51.0](https://crates.io/crates/wit-bindgen/0.51.0) |
| `writeable` | `0.6.3` | `Unicode-3.0` | `LICENSE` | [writeable 0.6.3](https://crates.io/crates/writeable/0.6.3) |
| `wry` | `0.53.5` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.spdx` | [wry 0.53.5](https://crates.io/crates/wry/0.53.5) |
| `x11` | `2.21.0` | `MIT` | `LICENSE-MIT` | [x11 2.21.0](https://crates.io/crates/x11/2.21.0) |
| `x11-dl` | `2.21.0` | `MIT` | `LICENSE-MIT` | [x11-dl 2.21.0](https://crates.io/crates/x11-dl/2.21.0) |
| `x11rb` | `0.13.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [x11rb 0.13.2](https://crates.io/crates/x11rb/0.13.2) |
| `x11rb-protocol` | `0.13.2` | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` | [x11rb-protocol 0.13.2](https://crates.io/crates/x11rb-protocol/0.13.2) |
| `xcursor` | `0.3.10` | `MIT` | `LICENSE` | [xcursor 0.3.10](https://crates.io/crates/xcursor/0.3.10) |
| `xkbcommon-dl` | `0.4.2` | `MIT` | `LICENSE` | [xkbcommon-dl 0.4.2](https://crates.io/crates/xkbcommon-dl/0.4.2) |
| `xkeysym` | `0.2.1` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [xkeysym 0.2.1](https://crates.io/crates/xkeysym/0.2.1) |
| `xml-rs` | `0.8.28` | `MIT` | `LICENSE` | [xml-rs 0.8.28](https://crates.io/crates/xml-rs/0.8.28) |
| `yoke` | `0.8.2` | `Unicode-3.0` | `LICENSE` | [yoke 0.8.2](https://crates.io/crates/yoke/0.8.2) |
| `yoke-derive` | `0.8.2` | `Unicode-3.0` | `LICENSE` | [yoke-derive 0.8.2](https://crates.io/crates/yoke-derive/0.8.2) |
| `zbus` | `5.14.0` | `MIT` | `LICENSE` | [zbus 5.14.0](https://crates.io/crates/zbus/5.14.0) |
| `zbus-lockstep` | `0.5.2` | `MIT` | `LICENSE-MIT` | [zbus-lockstep 0.5.2](https://crates.io/crates/zbus-lockstep/0.5.2) |
| `zbus-lockstep-macros` | `0.5.2` | `MIT` | `LICENSE-MIT` | [zbus-lockstep-macros 0.5.2](https://crates.io/crates/zbus-lockstep-macros/0.5.2) |
| `zbus_macros` | `5.14.0` | `MIT` | `LICENSE` | [zbus_macros 5.14.0](https://crates.io/crates/zbus_macros/5.14.0) |
| `zbus_names` | `4.3.1` | `MIT` | `LICENSE` | [zbus_names 4.3.1](https://crates.io/crates/zbus_names/4.3.1) |
| `zbus_xml` | `5.1.0` | `MIT` | `LICENSE` | [zbus_xml 5.1.0](https://crates.io/crates/zbus_xml/5.1.0) |
| `zerocopy` | `0.8.48` | `BSD-2-Clause OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` | [zerocopy 0.8.48](https://crates.io/crates/zerocopy/0.8.48) |
| `zerocopy-derive` | `0.8.48` | `BSD-2-Clause OR Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` | [zerocopy-derive 0.8.48](https://crates.io/crates/zerocopy-derive/0.8.48) |
| `zerofrom` | `0.1.7` | `Unicode-3.0` | `LICENSE` | [zerofrom 0.1.7](https://crates.io/crates/zerofrom/0.1.7) |
| `zerofrom-derive` | `0.1.7` | `Unicode-3.0` | `LICENSE` | [zerofrom-derive 0.1.7](https://crates.io/crates/zerofrom-derive/0.1.7) |
| `zeroize` | `1.8.2` | `Apache-2.0 OR MIT` | `LICENSE-APACHE`, `LICENSE-MIT` | [zeroize 1.8.2](https://crates.io/crates/zeroize/1.8.2) |
| `zerotrie` | `0.2.4` | `Unicode-3.0` | `LICENSE` | [zerotrie 0.2.4](https://crates.io/crates/zerotrie/0.2.4) |
| `zerovec` | `0.11.6` | `Unicode-3.0` | `LICENSE` | [zerovec 0.11.6](https://crates.io/crates/zerovec/0.11.6) |
| `zerovec-derive` | `0.11.3` | `Unicode-3.0` | `LICENSE` | [zerovec-derive 0.11.3](https://crates.io/crates/zerovec-derive/0.11.3) |
| `zip` | `0.6.6` | `MIT` | `LICENSE` | [zip 0.6.6](https://crates.io/crates/zip/0.6.6) |
| `zmij` | `1.0.21` | `MIT` | `LICENSE-MIT` | [zmij 1.0.21](https://crates.io/crates/zmij/1.0.21) |
| `zstd` | `0.11.2+zstd.1.5.2` | `MIT` | `LICENSE` | [zstd 0.11.2+zstd.1.5.2](https://crates.io/crates/zstd/0.11.2+zstd.1.5.2) |
| `zstd-safe` | `5.0.2+zstd.1.5.2` | `MIT/Apache-2.0` | `LICENSE`, `LICENSE.Apache-2.0`, `LICENSE.Mit` | [zstd-safe 5.0.2+zstd.1.5.2](https://crates.io/crates/zstd-safe/5.0.2+zstd.1.5.2) |
| `zstd-sys` | `2.0.16+zstd.1.5.7` | `MIT/Apache-2.0` | `LICENSE`, `LICENSE.Apache-2.0`, `LICENSE.BSD-3-Clause`, `LICENSE.Mit` | [zstd-sys 2.0.16+zstd.1.5.7](https://crates.io/crates/zstd-sys/2.0.16+zstd.1.5.7) |
| `zune-core` | `0.5.1` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [zune-core 0.5.1](https://crates.io/crates/zune-core/0.5.1) |
| `zune-jpeg` | `0.5.15` | `MIT OR Apache-2.0 OR Zlib` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` | [zune-jpeg 0.5.15](https://crates.io/crates/zune-jpeg/0.5.15) |
| `zvariant` | `5.10.0` | `MIT` | `LICENSE` | [zvariant 5.10.0](https://crates.io/crates/zvariant/5.10.0) |
| `zvariant_derive` | `5.10.0` | `MIT` | `LICENSE` | [zvariant_derive 5.10.0](https://crates.io/crates/zvariant_derive/5.10.0) |
| `zvariant_utils` | `3.3.0` | `MIT` | `LICENSE` | [zvariant_utils 3.3.0](https://crates.io/crates/zvariant_utils/3.3.0) |

## Release checklist

- KOKURAの自作コードにはルートの `LICENSE` を同梱する。
- このファイルを `Cargo.lock` と同じ変更単位で更新する。
- バイナリ配布では、表に記載された依存のLICENSE/COPYING/NOTICE本文を配布物へ保持する。
- Apache-2.0依存については、ライセンス本文、著作権・帰属表示、NOTICE、変更ファイル表示を確認する。
- OFL/Ubuntu-font、Unicode、BSD、MPL-2.0など、MIT以外の表記をMITへ置き換えない。
- ROMイメージ、抽出アセット、タイトル固有のデバッグ証跡は、この通知ファイルがあっても公開対象にならない。

## License expression notes

この一覧のライセンス表記は、Cargoパッケージのメタデータをそのまま転記しています。
実際の再配布条件は、各パッケージに含まれるLICENSE/COPYING/NOTICE本文を優先して確認してください。
不明な表記、欠落した本文、社内ポリシーに合わない依存が見つかった場合は、公開前に依存を更新・除外するか、法務確認を行ってください。

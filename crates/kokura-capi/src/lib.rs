//! C ABI の公開エントリポイント。
//!
//! Rustの所有権をそのままCへ漏らさず、opaque handle、明示的な解放関数、
//! UTF-8文字列の所有権移譲、固定レイアウトのFFI型を通してC/Python側へ
//! `kokura-core` と `kokura-debug` の機能を提供します。

pub mod bridge;
pub mod core;
pub mod debug;
pub mod ffi_types;
pub mod strings;

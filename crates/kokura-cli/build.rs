//! CLIのホスト固有ビルド設定。
//!
//! Windowsで大きなデバッグレポートや状態復元処理を実行するため、メイン
//! スレッドのスタックサイズをリンカへ伝えます。

fn main() {
    #[cfg(target_os = "windows")]
    {
        // The debug CLI keeps large report/session construction frames alive at once.
        // A larger main-thread stack keeps state-load/report workflows stable on Windows.
        println!("cargo:rustc-link-arg=/STACK:16777216");
    }
}

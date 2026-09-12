use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KitaqgbIntrinsicKind {
    SetTile8,
    SetTile16,
    SetTileBuffered,
    SetTileCgb,
    SetTileFlush,
    SetTile16Flush,
    FlushRows,
    WaitVBlank,
    Present,
    OamTransfer,
    CgbPalette,
    Input,
    FarMemcpy,
    BankGuard,
    TrapCheck,
    Memcpy,
    Memset,
    OamDma,
    Music,
    Vram,
    Bank,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KitaqgbIntrinsicMatch {
    pub kind: KitaqgbIntrinsicKind,
    pub canonical_name: String,
}

impl KitaqgbIntrinsicMatch {
    pub fn has_likely_visual_effect(&self) -> bool {
        matches!(
            self.kind,
            KitaqgbIntrinsicKind::SetTile8
                | KitaqgbIntrinsicKind::SetTile16
                | KitaqgbIntrinsicKind::SetTileBuffered
                | KitaqgbIntrinsicKind::SetTileCgb
                | KitaqgbIntrinsicKind::SetTileFlush
                | KitaqgbIntrinsicKind::SetTile16Flush
                | KitaqgbIntrinsicKind::FlushRows
                | KitaqgbIntrinsicKind::OamDma
                | KitaqgbIntrinsicKind::Vram
                | KitaqgbIntrinsicKind::Present
                | KitaqgbIntrinsicKind::OamTransfer
                | KitaqgbIntrinsicKind::CgbPalette
        )
    }

    pub fn is_flush_like(&self) -> bool {
        matches!(
            self.kind,
            KitaqgbIntrinsicKind::SetTileFlush
                | KitaqgbIntrinsicKind::SetTile16Flush
                | KitaqgbIntrinsicKind::FlushRows
                | KitaqgbIntrinsicKind::Present
                | KitaqgbIntrinsicKind::OamTransfer
        )
    }
}

pub fn classify_symbol_name(symbol: &str) -> Option<KitaqgbIntrinsicMatch> {
    let lower = symbol.to_ascii_lowercase();
    let is_exact_cgb_palette_helper = matches!(
        lower.as_str(),
        "cgb_bg_palette"
            | "cgb_obj_palette"
            | "cgb_bg_color"
            | "cgb_obj_color"
            | "cgb_bg_rgb"
            | "cgb_obj_rgb"
            | "cgb_bg_colors"
            | "cgb_obj_colors"
            | "cgb_bg_colors_raw"
            | "cgb_obj_colors_raw"
            | "cgb_rgb15"
            | "cgb__palette_color_slot"
            | "cgb__limit_color_count"
            | "cgb__write_color_bg_raw"
            | "cgb__write_color_obj_raw"
            | "cgb__write_colors_bg_raw"
            | "cgb__write_colors_obj_raw"
    );

    let kind = if lower.starts_with("kq_trap_check_") {
        KitaqgbIntrinsicKind::TrapCheck
    } else if lower.starts_with("kq_bank_ok_") || lower.starts_with("kq_sp_ok_") {
        KitaqgbIntrinsicKind::BankGuard
    } else if lower.starts_with("__kq_far_memcpy_bank") {
        KitaqgbIntrinsicKind::FarMemcpy
    } else if lower.contains("waitvblank") || lower.contains("vblankwait") {
        KitaqgbIntrinsicKind::WaitVBlank
    } else if lower.contains("presentscreen")
        || lower.contains("buildplayscreen")
        || lower.contains("drawtitlemenu")
        || lower.contains("lockdraw_flushvblank")
        || lower.contains("present")
        || lower.contains("buildscreen")
    {
        KitaqgbIntrinsicKind::Present
    } else if lower.contains("transferoam") || lower.contains("applyshadowtooam") {
        KitaqgbIntrinsicKind::OamTransfer
    } else if lower.contains("cgb_writebgpal")
        || lower.contains("cgb_initpalettes")
        || is_exact_cgb_palette_helper
    {
        KitaqgbIntrinsicKind::CgbPalette
    } else if lower.contains("pad_read")
        || lower.contains("updateinput")
        || lower.contains("repeatinput")
        || lower.contains("clearinput")
        || lower.contains("readjoy")
        || lower.contains("pollinput")
    {
        KitaqgbIntrinsicKind::Input
    } else if lower.contains("flushtile16rows")
        || lower.contains("flushtile16")
        || lower.contains("flush_rows")
    {
        KitaqgbIntrinsicKind::FlushRows
    } else if lower.contains("__settilebg16cgb") || lower.contains("__settile16cgb") {
        if lower.contains("flush") {
            KitaqgbIntrinsicKind::SetTile16Flush
        } else {
            KitaqgbIntrinsicKind::SetTileCgb
        }
    } else if lower.contains("__settileattr")
        || lower.contains("__settileatattr")
        || lower.contains("__settilewinattr")
        || lower.contains("__settilebgattr")
        || lower.contains("__settileatcgb")
        || lower.contains("__settilewincgb")
        || lower.contains("__settilebgcgb")
        || lower.contains("__settilecgb_bulk")
        || lower.contains("__settileattr_bulk")
        || lower == "__settilecgb"
        || lower.contains("__settilecgb_")
    {
        KitaqgbIntrinsicKind::SetTileCgb
    } else if lower.contains("__settilebg16_buf")
        || lower.contains("__settile16_buf")
        || lower.contains("settile16buffer")
    {
        KitaqgbIntrinsicKind::SetTileBuffered
    } else if lower.contains("__settilebg16") || lower.contains("__settile16") {
        if lower.contains("flush") {
            KitaqgbIntrinsicKind::SetTile16Flush
        } else {
            KitaqgbIntrinsicKind::SetTile16
        }
    } else if lower.contains("__settilecgb") {
        KitaqgbIntrinsicKind::SetTileCgb
    } else if lower.contains("__settile")
        || lower == "__settile_core"
        || lower == "__settile_bulk_fast_core"
    {
        if lower.contains("flush") {
            KitaqgbIntrinsicKind::SetTileFlush
        } else if lower.contains("buf") {
            KitaqgbIntrinsicKind::SetTileBuffered
        } else {
            KitaqgbIntrinsicKind::SetTile8
        }
    } else if lower.contains("memcpy") {
        KitaqgbIntrinsicKind::Memcpy
    } else if lower.contains("memset") {
        KitaqgbIntrinsicKind::Memset
    } else if (lower.contains("oam") && lower.contains("dma")) || lower.contains("__oamdma") {
        KitaqgbIntrinsicKind::OamDma
    } else if lower.contains("music")
        || lower.contains("sound")
        || lower.contains("audio")
        || lower.contains("wavechange")
        || lower.contains("pan")
        || lower.contains("sweep")
    {
        KitaqgbIntrinsicKind::Music
    } else if lower.contains("vram")
        || lower.contains("tilemap")
        || lower.contains("bgmap")
        || lower.contains("loadtileslcdoff")
        || lower.contains("screenfill")
        || lower.contains("puttile")
    {
        KitaqgbIntrinsicKind::Vram
    } else if lower.contains("__kq_thunk_b")
        || lower.contains("farcall")
        || lower.contains("callbank")
        || lower.contains("switch_rom_bank")
        || lower.contains("banked")
        || lower.contains("rombank")
        || lower.contains("bank")
    {
        KitaqgbIntrinsicKind::Bank
    } else {
        return None;
    };

    Some(KitaqgbIntrinsicMatch {
        kind,
        canonical_name: symbol.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{classify_symbol_name, KitaqgbIntrinsicKind};

    #[test]
    fn classifies_new_cgb_tile_intrinsics() {
        let attr = classify_symbol_name("__settileattr_bulk_fast").unwrap();
        assert!(matches!(attr.kind, KitaqgbIntrinsicKind::SetTileCgb));

        let cgb = classify_symbol_name("__settilebgcgb_unsafe").unwrap();
        assert!(matches!(cgb.kind, KitaqgbIntrinsicKind::SetTileCgb));

        let bulk = classify_symbol_name("__settilecgb_bulk_fast").unwrap();
        assert!(matches!(bulk.kind, KitaqgbIntrinsicKind::SetTileCgb));
    }

    #[test]
    fn classifies_new_input_and_farcall_names() {
        let repeat = classify_symbol_name("UpdateRepeatInput").unwrap();
        assert!(matches!(repeat.kind, KitaqgbIntrinsicKind::Input));

        let clear = classify_symbol_name("ClearInputState").unwrap();
        assert!(matches!(clear.kind, KitaqgbIntrinsicKind::Input));

        let farcall = classify_symbol_name("__farcall_ptr").unwrap();
        assert!(matches!(farcall.kind, KitaqgbIntrinsicKind::Bank));
    }

    #[test]
    fn classifies_cgb_palette_helper_names() {
        let modern = classify_symbol_name("cgb_bg_palette").unwrap();
        assert!(matches!(modern.kind, KitaqgbIntrinsicKind::CgbPalette));
    }

    #[test]
    fn does_not_classify_legacy_kq_cgb_palette_names() {
        assert!(classify_symbol_name("kq_cgb_bg_palette").is_none());
        assert!(classify_symbol_name("kq_cgb_obj_rgb").is_none());
    }
}

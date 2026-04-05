use gpui::{rgb, rgba, Rgba};

// ── Surface / background ─────────────────────────────────────────
pub fn bg_base() -> Rgba { rgb(0x111111) }
pub fn bg_surface() -> Rgba { rgb(0x191919) }
pub fn bg_elevated() -> Rgba { rgba(0xffffff05) }
pub fn bg_hover() -> Rgba { rgba(0xffffff08) }
pub fn bg_active() -> Rgba { rgba(0xffffff0c) }
pub fn bg_selected() -> Rgba { rgba(0xffffff0f) }
pub fn bg_input() -> Rgba { rgba(0xffffff06) }

// ── Borders ──────────────────────────────────────────────────────
pub fn border() -> Rgba { rgba(0xffffff0a) }
pub fn border_subtle() -> Rgba { rgba(0xffffff06) }
pub fn border_strong() -> Rgba { rgba(0xffffff0f) }

// ── Text ─────────────────────────────────────────────────────────
pub fn text_primary() -> Rgba { rgba(0xffffffee) }
pub fn text_secondary() -> Rgba { rgba(0xffffffcc) }
pub fn text_tertiary() -> Rgba { rgba(0xffffffaa) }
pub fn text_muted() -> Rgba { rgba(0xffffff88) }
pub fn text_dim() -> Rgba { rgba(0xffffff55) }
pub fn text_ghost() -> Rgba { rgba(0xffffff44) }
pub fn text_faint() -> Rgba { rgba(0xffffff33) }
pub fn text_invisible() -> Rgba { rgba(0xffffff1a) }

// ── Status indicators ────────────────────────────────────────────
pub fn status_green_dim() -> Rgba { rgba(0x4ade80aa) }
pub fn status_dim() -> Rgba { rgba(0xffffff1a) }

// Error
pub fn error_text() -> Rgba { rgba(0xEF4444FF) }
pub fn error_bg() -> Rgba { rgba(0x1A0505FF) }
pub fn error_border() -> Rgba { rgba(0x3B1111FF) }

// ── Git file status ──────────────────────────────────────────────
pub fn status_modified() -> Rgba { rgba(0xe5c07bff) }
pub fn status_added() -> Rgba { rgba(0x98c379ff) }
pub fn status_deleted() -> Rgba { rgba(0xe06c75ff) }

// ── Diff viewer ─────────────────────────────────────────────────
pub fn diff_add_bg() -> Rgba { rgba(0x98c37930) }
pub fn diff_del_bg() -> Rgba { rgba(0xe06c7530) }
pub fn diff_add_text() -> Rgba { rgba(0x98c379cc) }
pub fn diff_del_text() -> Rgba { rgba(0xe06c75cc) }
pub fn diff_hunk_header() -> Rgba { rgba(0x61afef66) }

// ── Utilities ────────────────────────────────────────────────────
pub fn parse_hex_color(hex: &str) -> Option<Rgba> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    })
}

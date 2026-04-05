//! Reusable animation presets built on GPUI's animation system.

use gpui::*;
use std::time::Duration;

/// Breathing opacity animation — smooth sine-based pulse between 0.3 and 1.0.
pub fn breathing(duration_secs: f32) -> Animation {
    Animation::new(Duration::from_secs_f32(duration_secs))
        .repeat()
        .with_easing(pulsating_between(0.3, 1.0))
}

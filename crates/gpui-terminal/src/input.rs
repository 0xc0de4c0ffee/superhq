//! Keyboard input handling for the terminal emulator.
//!
//! This module provides [`keystroke_to_bytes`], which converts GPUI keyboard
//! events into terminal escape sequences that can be written to the PTY.
//!
//! # Key Mappings
//!
//! ## Special Keys
//!
//! | Key | Sequence | Notes |
//! |-----|----------|-------|
//! | Enter | `\r` (0x0D) | Carriage return |
//! | Escape | `\x1b` (0x1B) | ESC |
//! | Backspace | `\x7f` (0x7F) | DEL |
//! | Tab | `\t` (0x09) | Horizontal tab |
//! | Shift+Tab | `\x1b[Z` | Backtab |
//! | Space | ` ` (0x20) | Space |
//! | Ctrl+Space | `\x00` | NUL |
//!
//! ## Arrow Keys
//!
//! Arrow key sequences depend on application cursor mode:
//!
//! | Key | Normal Mode | App Cursor Mode |
//! |-----|-------------|-----------------|
//! | Up | `\x1b[A` | `\x1bOA` |
//! | Down | `\x1b[B` | `\x1bOB` |
//! | Right | `\x1b[C` | `\x1bOC` |
//! | Left | `\x1b[D` | `\x1bOD` |
//!
//! ## Navigation Keys
//!
//! | Key | Sequence |
//! |-----|----------|
//! | Home | `\x1b[H` |
//! | End | `\x1b[F` |
//! | PageUp | `\x1b[5~` |
//! | PageDown | `\x1b[6~` |
//! | Insert | `\x1b[2~` |
//! | Delete | `\x1b[3~` |
//!
//! ## Function Keys
//!
//! | Key | Sequence |
//! |-----|----------|
//! | F1-F4 | `\x1bOP` - `\x1bOS` |
//! | F5-F12 | `\x1b[15~` - `\x1b[24~` |
//!
//! ## Control Combinations
//!
//! Ctrl+A through Ctrl+Z map to ASCII control characters 0x01-0x1A:
//!
//! | Combination | Byte |
//! |-------------|------|
//! | Ctrl+A | 0x01 |
//! | Ctrl+C | 0x03 (interrupt) |
//! | Ctrl+D | 0x04 (EOF) |
//! | Ctrl+Z | 0x1A (suspend) |
//!
//! ## Alt Combinations
//!
//! Alt+key sends ESC followed by the key: `\x1b` + key
//!
//! # Terminal Mode Effects
//!
//! The [`TermMode`] flags affect key sequences:
//!
//! - **APP_CURSOR**: Changes arrow key sequences from CSI to SS3 format
//!
//! # Example
//!
//! ```
//! use gpui::Keystroke;
//! use alacritty_terminal::term::TermMode;
//! use gpui_terminal::input::keystroke_to_bytes;
//!
//! // Enter key
//! let keystroke = Keystroke::parse("enter").unwrap();
//! assert_eq!(keystroke_to_bytes(&keystroke, TermMode::empty()), Some(b"\r".to_vec()));
//!
//! // Ctrl+C (interrupt)
//! let keystroke = Keystroke::parse("ctrl-c").unwrap();
//! assert_eq!(keystroke_to_bytes(&keystroke, TermMode::empty()), Some(vec![0x03]));
//! ```

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

/// Convert a GPUI keystroke to terminal escape sequence bytes.
///
/// This function translates GPUI keyboard events into the appropriate byte sequences
/// expected by terminal applications. It handles special keys, control characters,
/// and application cursor mode.
///
/// # Arguments
///
/// * `keystroke` - The GPUI keystroke to convert
/// * `mode` - The current terminal mode (affects arrow key sequences)
///
/// # Returns
///
/// An optional vector of bytes representing the terminal escape sequence.
/// Returns `None` if the keystroke should not produce any output.
///
/// # Examples
///
/// ```
/// use gpui::Keystroke;
/// use alacritty_terminal::term::TermMode;
/// use gpui_terminal::input::keystroke_to_bytes;
///
/// let keystroke = Keystroke::parse("enter").unwrap();
/// let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
/// assert_eq!(bytes, Some(b"\r".to_vec()));
/// ```
/// Compute the xterm modifier code for a keystroke.
/// Shift=2, Alt=3, Shift+Alt=4, Ctrl=5, Shift+Ctrl=6, Alt+Ctrl=7, Shift+Alt+Ctrl=8.
/// Returns None if no modifiers are held.
fn modifier_code(keystroke: &Keystroke) -> Option<u8> {
    let mut code: u8 = 1;
    if keystroke.modifiers.shift {
        code += 1;
    }
    if keystroke.modifiers.alt {
        code += 2;
    }
    if keystroke.modifiers.control {
        code += 4;
    }
    if code > 1 { Some(code) } else { None }
}

/// Build a CSI sequence with modifier: ESC [ 1 ; <mod> <suffix>
fn modified_csi(modifier: u8, suffix: &str) -> Vec<u8> {
    format!("\x1b[1;{modifier}{suffix}").into_bytes()
}

/// Build a tilde-key sequence with modifier: ESC [ <num> ; <mod> ~
fn modified_tilde(num: u8, modifier: u8) -> Vec<u8> {
    format!("\x1b[{num};{modifier}~").into_bytes()
}

pub fn keystroke_to_bytes(keystroke: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    let mods = &keystroke.modifiers;
    let key = keystroke.key.as_str();

    // ── Stage 0: Platform/Cmd modifier ──────────────────────────────
    // macOS navigation shortcuts that map to shell control sequences,
    // matching Alacritty/Ghostty/superset behavior.
    if mods.platform {
        return match key {
            "k" => Some(b"\x0c".to_vec()),              // Cmd+K: clear (form feed)
            "left" => Some(b"\x01".to_vec()),            // Cmd+Left: line start (Ctrl+A)
            "right" => Some(b"\x05".to_vec()),           // Cmd+Right: line end (Ctrl+E)
            "backspace" => Some(b"\x15".to_vec()),       // Cmd+Backspace: clear to start (Ctrl+U)
            _ => None,                                    // Pass to app
        };
    }

    // ── Stage 1: Special keys with modifier awareness ───────────────
    let mod_code = modifier_code(keystroke);

    match key {
        // Basic control characters
        "space" => {
            if mods.control {
                return Some(b"\x00".to_vec());
            }
            return Some(b" ".to_vec());
        }
        "enter" => {
            if mods.shift {
                return Some(b"\x1b\r".to_vec()); // Shift+Enter: ESC + CR
            }
            return Some(b"\r".to_vec());
        }
        "escape" => return Some(b"\x1b".to_vec()),
        "backspace" => {
            if mods.alt {
                return Some(b"\x17".to_vec()); // Option+Backspace: delete word (Ctrl+W)
            }
            return Some(b"\x7f".to_vec());
        }
        "tab" => {
            if mods.shift {
                return Some(b"\x1b[Z".to_vec());
            }
            return Some(b"\t".to_vec());
        }

        // Arrow keys: modifier-aware (Zed 3-stage pattern)
        "up" => return Some(match mod_code {
            Some(m) => modified_csi(m, "A"),
            None if mode.contains(TermMode::APP_CURSOR) => b"\x1bOA".to_vec(),
            None => b"\x1b[A".to_vec(),
        }),
        "down" => return Some(match mod_code {
            Some(m) => modified_csi(m, "B"),
            None if mode.contains(TermMode::APP_CURSOR) => b"\x1bOB".to_vec(),
            None => b"\x1b[B".to_vec(),
        }),
        "right" => return Some(match mod_code {
            Some(m) => modified_csi(m, "C"),
            None if mode.contains(TermMode::APP_CURSOR) => b"\x1bOC".to_vec(),
            None => b"\x1b[C".to_vec(),
        }),
        "left" => return Some(match mod_code {
            Some(m) => modified_csi(m, "D"),
            None if mode.contains(TermMode::APP_CURSOR) => b"\x1bOD".to_vec(),
            None => b"\x1b[D".to_vec(),
        }),

        // Navigation keys: modifier-aware
        "home" => return Some(match mod_code {
            Some(m) => modified_csi(m, "H"),
            None => b"\x1b[H".to_vec(),
        }),
        "end" => return Some(match mod_code {
            Some(m) => modified_csi(m, "F"),
            None => b"\x1b[F".to_vec(),
        }),
        "pageup" => return Some(match mod_code {
            Some(m) => modified_tilde(5, m),
            None => b"\x1b[5~".to_vec(),
        }),
        "pagedown" => return Some(match mod_code {
            Some(m) => modified_tilde(6, m),
            None => b"\x1b[6~".to_vec(),
        }),
        "insert" => return Some(match mod_code {
            Some(m) => modified_tilde(2, m),
            None => b"\x1b[2~".to_vec(),
        }),
        "delete" => return Some(match mod_code {
            Some(m) => modified_tilde(3, m),
            None => b"\x1b[3~".to_vec(),
        }),

        // Function keys
        "f1" => return Some(match mod_code {
            Some(m) => modified_csi(m, "P"),
            None => b"\x1bOP".to_vec(),
        }),
        "f2" => return Some(match mod_code {
            Some(m) => modified_csi(m, "Q"),
            None => b"\x1bOQ".to_vec(),
        }),
        "f3" => return Some(match mod_code {
            Some(m) => modified_csi(m, "R"),
            None => b"\x1bOR".to_vec(),
        }),
        "f4" => return Some(match mod_code {
            Some(m) => modified_csi(m, "S"),
            None => b"\x1bOS".to_vec(),
        }),
        "f5" => return Some(match mod_code {
            Some(m) => modified_tilde(15, m),
            None => b"\x1b[15~".to_vec(),
        }),
        "f6" => return Some(match mod_code {
            Some(m) => modified_tilde(17, m),
            None => b"\x1b[17~".to_vec(),
        }),
        "f7" => return Some(match mod_code {
            Some(m) => modified_tilde(18, m),
            None => b"\x1b[18~".to_vec(),
        }),
        "f8" => return Some(match mod_code {
            Some(m) => modified_tilde(19, m),
            None => b"\x1b[19~".to_vec(),
        }),
        "f9" => return Some(match mod_code {
            Some(m) => modified_tilde(20, m),
            None => b"\x1b[20~".to_vec(),
        }),
        "f10" => return Some(match mod_code {
            Some(m) => modified_tilde(21, m),
            None => b"\x1b[21~".to_vec(),
        }),
        "f11" => return Some(match mod_code {
            Some(m) => modified_tilde(23, m),
            None => b"\x1b[23~".to_vec(),
        }),
        "f12" => return Some(match mod_code {
            Some(m) => modified_tilde(24, m),
            None => b"\x1b[24~".to_vec(),
        }),

        _ => {}
    }

    // ── Stage 2: Ctrl+key combinations ──────────────────────────────
    if mods.control {
        if key.len() == 1 {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii_alphabetic() {
                let ctrl_char = (ch.to_ascii_uppercase() as u8) - b'@';
                return Some(vec![ctrl_char]);
            }
            match ch {
                '[' => return Some(b"\x1b".to_vec()),
                '\\' => return Some(b"\x1c".to_vec()),
                ']' => return Some(b"\x1d".to_vec()),
                '^' => return Some(b"\x1e".to_vec()),
                '_' => return Some(b"\x1f".to_vec()),
                '?' => return Some(b"\x7f".to_vec()),
                _ => {}
            }
        }
    }

    // ── Stage 3: Alt/Option key combinations ────────────────────────
    // Option+key sends ESC followed by the key (meta convention).
    // Option+Left/Right handled above as word navigation.
    if mods.alt {
        if key.len() == 1 {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii() {
                return Some(vec![b'\x1b', ch as u8]);
            }
        }
    }

    // ── Stage 4: Regular printable characters ───────────────────────
    if let Some(key_char) = &keystroke.key_char
        && !mods.control
        && !mods.alt
    {
        return Some(key_char.as_bytes().to_vec());
    }

    if key.len() == 1 {
        let ch = key.chars().next().unwrap();
        if ch.is_ascii() && !mods.control {
            let ch = if mods.shift {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            return Some(vec![ch as u8]);
        }
        if !mods.control && !mods.alt {
            return Some(key.as_bytes().to_vec());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_key() {
        let keystroke = Keystroke::parse("enter").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\r".to_vec()));
    }

    #[test]
    fn test_escape_key() {
        let keystroke = Keystroke::parse("escape").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x1b".to_vec()));
    }

    #[test]
    fn test_backspace_key() {
        let keystroke = Keystroke::parse("backspace").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x7f".to_vec()));
    }

    #[test]
    fn test_tab_key() {
        let keystroke = Keystroke::parse("tab").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\t".to_vec()));
    }

    #[test]
    fn test_shift_tab() {
        let keystroke = Keystroke::parse("shift-tab").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_arrow_keys_normal_mode() {
        let mode = TermMode::empty();

        let up = Keystroke::parse("up").unwrap();
        assert_eq!(keystroke_to_bytes(&up, mode), Some(b"\x1b[A".to_vec()));

        let down = Keystroke::parse("down").unwrap();
        assert_eq!(keystroke_to_bytes(&down, mode), Some(b"\x1b[B".to_vec()));

        let right = Keystroke::parse("right").unwrap();
        assert_eq!(keystroke_to_bytes(&right, mode), Some(b"\x1b[C".to_vec()));

        let left = Keystroke::parse("left").unwrap();
        assert_eq!(keystroke_to_bytes(&left, mode), Some(b"\x1b[D".to_vec()));
    }

    #[test]
    fn test_arrow_keys_app_cursor_mode() {
        let mode = TermMode::APP_CURSOR;

        let up = Keystroke::parse("up").unwrap();
        assert_eq!(keystroke_to_bytes(&up, mode), Some(b"\x1bOA".to_vec()));

        let down = Keystroke::parse("down").unwrap();
        assert_eq!(keystroke_to_bytes(&down, mode), Some(b"\x1bOB".to_vec()));

        let right = Keystroke::parse("right").unwrap();
        assert_eq!(keystroke_to_bytes(&right, mode), Some(b"\x1bOC".to_vec()));

        let left = Keystroke::parse("left").unwrap();
        assert_eq!(keystroke_to_bytes(&left, mode), Some(b"\x1bOD".to_vec()));
    }

    #[test]
    fn test_navigation_keys() {
        let mode = TermMode::empty();

        let home = Keystroke::parse("home").unwrap();
        assert_eq!(keystroke_to_bytes(&home, mode), Some(b"\x1b[H".to_vec()));

        let end = Keystroke::parse("end").unwrap();
        assert_eq!(keystroke_to_bytes(&end, mode), Some(b"\x1b[F".to_vec()));

        let pageup = Keystroke::parse("pageup").unwrap();
        assert_eq!(keystroke_to_bytes(&pageup, mode), Some(b"\x1b[5~".to_vec()));

        let pagedown = Keystroke::parse("pagedown").unwrap();
        assert_eq!(
            keystroke_to_bytes(&pagedown, mode),
            Some(b"\x1b[6~".to_vec())
        );

        let insert = Keystroke::parse("insert").unwrap();
        assert_eq!(keystroke_to_bytes(&insert, mode), Some(b"\x1b[2~".to_vec()));

        let delete = Keystroke::parse("delete").unwrap();
        assert_eq!(keystroke_to_bytes(&delete, mode), Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn test_function_keys() {
        let mode = TermMode::empty();

        let f1 = Keystroke::parse("f1").unwrap();
        assert_eq!(keystroke_to_bytes(&f1, mode), Some(b"\x1bOP".to_vec()));

        let f2 = Keystroke::parse("f2").unwrap();
        assert_eq!(keystroke_to_bytes(&f2, mode), Some(b"\x1bOQ".to_vec()));

        let f5 = Keystroke::parse("f5").unwrap();
        assert_eq!(keystroke_to_bytes(&f5, mode), Some(b"\x1b[15~".to_vec()));

        let f12 = Keystroke::parse("f12").unwrap();
        assert_eq!(keystroke_to_bytes(&f12, mode), Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn test_ctrl_combinations() {
        let mode = TermMode::empty();

        // Ctrl+A = 0x01
        let ctrl_a = Keystroke::parse("ctrl-a").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_a, mode), Some(vec![0x01]));

        // Ctrl+C = 0x03
        let ctrl_c = Keystroke::parse("ctrl-c").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_c, mode), Some(vec![0x03]));

        // Ctrl+Z = 0x1a
        let ctrl_z = Keystroke::parse("ctrl-z").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_z, mode), Some(vec![0x1a]));

        // Ctrl+Space = 0x00
        let ctrl_space = Keystroke::parse("ctrl-space").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_space, mode), Some(vec![0x00]));
    }

    #[test]
    fn test_alt_combinations() {
        let mode = TermMode::empty();

        // Alt+a sends ESC followed by 'a'
        let alt_a = Keystroke::parse("alt-a").unwrap();
        assert_eq!(keystroke_to_bytes(&alt_a, mode), Some(b"\x1ba".to_vec()));

        // Alt+x sends ESC followed by 'x'
        let alt_x = Keystroke::parse("alt-x").unwrap();
        assert_eq!(keystroke_to_bytes(&alt_x, mode), Some(b"\x1bx".to_vec()));
    }

    #[test]
    fn test_regular_characters() {
        let mode = TermMode::empty();

        let a = Keystroke::parse("a").unwrap();
        assert_eq!(keystroke_to_bytes(&a, mode), Some(b"a".to_vec()));

        let z = Keystroke::parse("z").unwrap();
        assert_eq!(keystroke_to_bytes(&z, mode), Some(b"z".to_vec()));

        let zero = Keystroke::parse("0").unwrap();
        assert_eq!(keystroke_to_bytes(&zero, mode), Some(b"0".to_vec()));
    }

    #[test]
    fn test_space_key() {
        let mode = TermMode::empty();

        let space = Keystroke::parse("space").unwrap();
        assert_eq!(keystroke_to_bytes(&space, mode), Some(b" ".to_vec()));
    }
}

use gpui::{actions, App, KeyBinding};

actions!(superhq, [Cancel, Confirm, SelectUp, SelectDown]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        // Context menu
        KeyBinding::new("escape", Cancel, Some("ContextMenu")),
        KeyBinding::new("up", SelectUp, Some("ContextMenu")),
        KeyBinding::new("down", SelectDown, Some("ContextMenu")),
        KeyBinding::new("enter", Confirm, Some("ContextMenu")),
        // Select / dropdown
        KeyBinding::new("escape", Cancel, Some("Select")),
        KeyBinding::new("up", SelectUp, Some("Select")),
        KeyBinding::new("down", SelectDown, Some("Select")),
        KeyBinding::new("enter", Confirm, Some("Select")),
        // Text input
        KeyBinding::new("escape", Cancel, Some("TextInput")),
        // Button
        KeyBinding::new("enter", Confirm, Some("Button")),
        KeyBinding::new("space", Confirm, Some("Button")),
        // Dialog
        KeyBinding::new("escape", Cancel, Some("Dialog")),
        KeyBinding::new("enter", Confirm, Some("Dialog")),
    ]);
}

//! Overlay interaction mode and the Ctrl+O toggle.
//!
//! The overlay has two states:
//!
//! - [`Mode::View`] (default): the surface is **click-through**. Every pointer
//!   and key event goes to the game underneath; the user only *sees* the
//!   panels. Content keeps running regardless (a captured video keeps playing,
//!   with its own audio), the mode only decides who receives input, never
//!   whether sources are live.
//! - [`Mode::Editor`]: opened with **Ctrl+O** (a global hotkey, so it fires
//!   even while the surface is click-through). The surface now captures input;
//!   the user arranges, scales and crops panels.
//!
//! [`InputRouting`] is what each mode means for the platform window layer: in
//! View the input region is empty (the compositor must set it so), in Editor it
//! is the whole surface.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    View,
    Editor,
}

/// What the window backend must enforce for a given mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputRouting {
    /// `true` => the overlay swallows input (Editor). `false` => input passes
    /// through to the game (View): the platform layer sets an empty input
    /// region / `WS_EX_TRANSPARENT` / `ignoresMouseEvents`.
    pub interactive: bool,
    /// Whether the editor chrome (handles, toolbar) should be drawn.
    pub show_editor: bool,
}

impl Mode {
    pub fn toggled(self) -> Self {
        match self {
            Mode::View => Mode::Editor,
            Mode::Editor => Mode::View,
        }
    }

    pub fn routing(self) -> InputRouting {
        match self {
            Mode::View => InputRouting {
                interactive: false,
                show_editor: false,
            },
            Mode::Editor => InputRouting {
                interactive: true,
                show_editor: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_click_through_view() {
        let r = Mode::default().routing();
        assert!(!r.interactive);
        assert!(!r.show_editor);
    }

    #[test]
    fn toggle_round_trips() {
        assert_eq!(Mode::View.toggled(), Mode::Editor);
        assert_eq!(Mode::View.toggled().toggled(), Mode::View);
    }

    #[test]
    fn editor_is_interactive() {
        assert!(Mode::Editor.routing().interactive);
    }
}

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::BorderType;

/// Modern, vibrant color palette for Amixima
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub border_inactive: Color,
    pub border_active: Color,
    pub accent_fuchsia: Color,
    pub accent_cyan: Color,
    pub accent_gold: Color,
    pub accent_green: Color,
    pub selection_bg: Color,
    pub text_bright: Color,
    pub text_dim: Color,
    pub error: Color,
    pub success: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color::Rgb(15, 15, 20), // Deep space blue-black
            surface: Color::Rgb(25, 25, 35),    // Dark slate gray-blue
            border_inactive: Color::Rgb(50, 50, 70),
            border_active: Color::Rgb(129, 140, 248), // Indigo-ish
            accent_fuchsia: Color::Rgb(217, 70, 239),
            accent_cyan: Color::Rgb(34, 211, 238),
            accent_gold: Color::Rgb(251, 191, 36),
            accent_green: Color::Rgb(52, 211, 153),
            selection_bg: Color::Rgb(45, 45, 70),
            text_bright: Color::Rgb(248, 250, 252),
            text_dim: Color::Rgb(148, 163, 184),
            error: Color::Rgb(239, 68, 68),
            success: Color::Rgb(34, 197, 94),
        }
    }
}

/// Decorative symbols for a modern TUI look
pub struct Symbols;

impl Symbols {
    pub const FOCUS_MARKER: &'static str = "➤ ";
    pub const ITEM_MARKER: &'static str = "◆ ";
    pub const FILE_ICON: &'static str = "󰈚 "; // Using standard symbols if font supports, else alternatives
    pub const DIR_ICON: &'static str = "󰉋 ";
    pub const AUDIO_ICON: &'static str = "󰎆 ";
    pub const CONFIG_ICON: &'static str = "󱁻 ";

    // Plain text replacements for broad compatibility but high style
    pub const NAV_INDICATOR: &'static str = " » ";
}

pub struct StyleManager {
    pub palette: Palette,
}

impl StyleManager {
    pub fn new() -> Self {
        Self {
            palette: Palette::default(),
        }
    }

    pub fn block_style(&self, active: bool) -> Style {
        if active {
            Style::default().fg(self.palette.border_active)
        } else {
            Style::default().fg(self.palette.border_inactive)
        }
    }

    pub fn title_style(&self, active: bool) -> Style {
        if active {
            Style::default()
                .fg(self.palette.accent_cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.palette.text_dim)
                .add_modifier(Modifier::DIM)
        }
    }

    pub fn list_highlight_style(&self, active: bool) -> Style {
        if active {
            Style::default()
                .bg(self.palette.selection_bg)
                .fg(self.palette.accent_fuchsia)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(self.palette.selection_bg)
                .fg(self.palette.text_dim)
        }
    }

    pub fn border_type(&self, active: bool) -> BorderType {
        if active {
            BorderType::Thick
        } else {
            BorderType::Rounded
        }
    }
}

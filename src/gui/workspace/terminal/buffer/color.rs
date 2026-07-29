use alacritty_terminal::{
    term::{cell::Flags, color::Colors},
    vte::ansi::{Color, NamedColor, Rgb},
};

use crate::domain::terminal::TerminalRgb;

pub(super) fn resolve_color(
    color: Color,
    overrides: &Colors,
    foreground: bool,
    flags: Flags,
) -> TerminalRgb {
    let color = match color {
        Color::Named(named) if foreground && flags.contains(Flags::BOLD) => {
            Color::Named(named.to_bright())
        }
        Color::Named(named) if foreground && flags.contains(Flags::DIM) => {
            Color::Named(named.to_dim())
        }
        color => color,
    };
    match color {
        Color::Spec(rgb) => rgb.into(),
        Color::Indexed(index) => overrides[index as usize]
            .map(Into::into)
            .unwrap_or_else(|| indexed_color(index)),
        Color::Named(named) => overrides[named]
            .map(Into::into)
            .unwrap_or_else(|| named_color(named)),
    }
}

impl From<Rgb> for TerminalRgb {
    fn from(value: Rgb) -> Self {
        Self {
            red: value.r,
            green: value.g,
            blue: value.b,
        }
    }
}

fn default_foreground() -> TerminalRgb {
    TerminalRgb {
        red: 226,
        green: 232,
        blue: 240,
    }
}

pub(in crate::gui::workspace::terminal) fn default_background() -> TerminalRgb {
    TerminalRgb {
        red: 20,
        green: 18,
        blue: 24,
    }
}

fn named_color(color: NamedColor) -> TerminalRgb {
    match color {
        NamedColor::Foreground | NamedColor::BrightForeground => default_foreground(),
        NamedColor::Background => default_background(),
        NamedColor::Cursor => default_foreground(),
        NamedColor::DimForeground => dim(default_foreground()),
        NamedColor::Black => indexed_color(0),
        NamedColor::Red => indexed_color(1),
        NamedColor::Green => indexed_color(2),
        NamedColor::Yellow => indexed_color(3),
        NamedColor::Blue => indexed_color(4),
        NamedColor::Magenta => indexed_color(5),
        NamedColor::Cyan => indexed_color(6),
        NamedColor::White => indexed_color(7),
        NamedColor::BrightBlack => indexed_color(8),
        NamedColor::BrightRed => indexed_color(9),
        NamedColor::BrightGreen => indexed_color(10),
        NamedColor::BrightYellow => indexed_color(11),
        NamedColor::BrightBlue => indexed_color(12),
        NamedColor::BrightMagenta => indexed_color(13),
        NamedColor::BrightCyan => indexed_color(14),
        NamedColor::BrightWhite => indexed_color(15),
        NamedColor::DimBlack => dim(indexed_color(0)),
        NamedColor::DimRed => dim(indexed_color(1)),
        NamedColor::DimGreen => dim(indexed_color(2)),
        NamedColor::DimYellow => dim(indexed_color(3)),
        NamedColor::DimBlue => dim(indexed_color(4)),
        NamedColor::DimMagenta => dim(indexed_color(5)),
        NamedColor::DimCyan => dim(indexed_color(6)),
        NamedColor::DimWhite => dim(indexed_color(7)),
    }
}

fn indexed_color(index: u8) -> TerminalRgb {
    const ANSI: [[u8; 3]; 16] = [
        [30, 30, 30],
        [205, 49, 49],
        [13, 188, 121],
        [229, 229, 16],
        [36, 114, 200],
        [188, 63, 188],
        [17, 168, 205],
        [229, 229, 229],
        [102, 102, 102],
        [241, 76, 76],
        [35, 209, 139],
        [245, 245, 67],
        [59, 142, 234],
        [214, 112, 214],
        [41, 184, 219],
        [255, 255, 255],
    ];
    if index < 16 {
        let [red, green, blue] = ANSI[index as usize];
        return TerminalRgb { red, green, blue };
    }
    if index < 232 {
        let value = index - 16;
        let component = |part: u8| {
            if part == 0 { 0 } else { 55 + part * 40 }
        };
        return TerminalRgb {
            red: component(value / 36),
            green: component((value % 36) / 6),
            blue: component(value % 6),
        };
    }
    let gray = 8 + (index - 232) * 10;
    TerminalRgb {
        red: gray,
        green: gray,
        blue: gray,
    }
}

fn dim(color: TerminalRgb) -> TerminalRgb {
    TerminalRgb {
        red: color.red.saturating_mul(2) / 3,
        green: color.green.saturating_mul(2) / 3,
        blue: color.blue.saturating_mul(2) / 3,
    }
}

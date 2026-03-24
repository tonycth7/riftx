use ratatui::style::Color;
use crate::config::ThemeName;

/// A complete color palette used throughout the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    // Backgrounds
    pub bg:      Color,
    pub bg2:     Color,
    pub bg3:     Color,
    #[allow(dead_code)]
    pub bg_sel:  Color,
    // Text
    pub bright:  Color,
    pub mid:     Color,
    pub dim:     Color,
    pub vdim:    Color,
    // Accent — primary / secondary / tertiary
    pub accent:  Color,
    pub accent2: Color,
    pub accent3: Color,
    // Semantic
    pub green:   Color,
    pub blue:    Color,
    pub red:     Color,
    pub yellow:  Color,
    pub purple:  Color,
    pub cyan:    Color,
    pub orange:  Color,
    pub teal:    Color,
    #[allow(dead_code)]
    pub pink:    Color,
    // Fuzzy match highlight
    pub match_hl: Color,
}

impl Theme {
    pub fn get(name: &ThemeName) -> Self {
        match name {
            ThemeName::Amber      => amber(),
            ThemeName::Dracula    => dracula(),
            ThemeName::Nord       => nord(),
            ThemeName::Gruvbox    => gruvbox(),
            ThemeName::Catppuccin => catppuccin(),
            ThemeName::SkyBlue    => skyblue(),
            ThemeName::TokyoNight => tokyonight(),
            ThemeName::Ayu        => ayu(),
        }
    }
}

// ─── Amber (default) ──────────────────────────────────────────────────────────
fn amber() -> Theme {
    Theme {
        bg:       Color::Rgb( 11,  11,  14),
        bg2:      Color::Rgb( 16,  16,  22),
        bg3:      Color::Rgb( 22,  22,  30),
        bg_sel:   Color::Rgb( 18,  24,  42),
        bright:   Color::Rgb(226, 232, 240),
        mid:      Color::Rgb(148, 163, 184),
        dim:      Color::Rgb( 71,  71,  90),
        vdim:     Color::Rgb( 30,  30,  46),
        accent:   Color::Rgb(245, 158,  11),
        accent2:  Color::Rgb(180, 110,   5),
        accent3:  Color::Rgb( 80,  48,   2),
        green:    Color::Rgb( 74, 222, 128),
        blue:     Color::Rgb( 96, 165, 250),
        red:      Color::Rgb(248, 113, 113),
        yellow:   Color::Rgb(250, 204,  21),
        purple:   Color::Rgb(167, 139, 250),
        cyan:     Color::Rgb(103, 232, 249),
        orange:   Color::Rgb(249, 115,  22),
        teal:     Color::Rgb( 52, 211, 153),
        pink:     Color::Rgb(244, 114, 182),
        match_hl: Color::Rgb(245, 158,  11),
    }
}

// ─── Dracula ──────────────────────────────────────────────────────────────────
fn dracula() -> Theme {
    Theme {
        bg:       Color::Rgb( 40,  42,  54),
        bg2:      Color::Rgb( 33,  34,  44),
        bg3:      Color::Rgb( 68,  71,  90),
        bg_sel:   Color::Rgb( 68,  71,  90),
        bright:   Color::Rgb(248, 248, 242),
        mid:      Color::Rgb(191, 193, 202),
        dim:      Color::Rgb(130, 132, 145),
        vdim:     Color::Rgb( 68,  71,  90),
        accent:   Color::Rgb(255, 121, 198),
        accent2:  Color::Rgb(189,  87, 158),
        accent3:  Color::Rgb( 80,  30,  60),
        green:    Color::Rgb( 80, 250, 123),
        blue:     Color::Rgb(102, 217, 239),
        red:      Color::Rgb(255,  85,  85),
        yellow:   Color::Rgb(241, 250, 140),
        purple:   Color::Rgb(189, 147, 249),
        cyan:     Color::Rgb(139, 233, 253),
        orange:   Color::Rgb(255, 184,  84),
        teal:     Color::Rgb( 80, 250, 123),
        pink:     Color::Rgb(255, 121, 198),
        match_hl: Color::Rgb(255, 184,  84),
    }
}

// ─── Nord ─────────────────────────────────────────────────────────────────────
fn nord() -> Theme {
    Theme {
        bg:       Color::Rgb( 46,  52,  64),
        bg2:      Color::Rgb( 59,  66,  82),
        bg3:      Color::Rgb( 67,  76,  94),
        bg_sel:   Color::Rgb( 76,  86, 106),
        bright:   Color::Rgb(236, 239, 244),
        mid:      Color::Rgb(216, 222, 233),
        dim:      Color::Rgb(129, 161, 193),
        vdim:     Color::Rgb( 76,  86, 106),
        accent:   Color::Rgb(136, 192, 208),
        accent2:  Color::Rgb( 94, 129, 172),
        accent3:  Color::Rgb( 59,  66,  82),
        green:    Color::Rgb(163, 190, 140),
        blue:     Color::Rgb(129, 161, 193),
        red:      Color::Rgb(191,  97, 106),
        yellow:   Color::Rgb(235, 203, 139),
        purple:   Color::Rgb(180, 142, 173),
        cyan:     Color::Rgb(143, 188, 187),
        orange:   Color::Rgb(208, 135, 112),
        teal:     Color::Rgb(143, 188, 187),
        pink:     Color::Rgb(180, 142, 173),
        match_hl: Color::Rgb(235, 203, 139),
    }
}

// ─── Gruvbox ──────────────────────────────────────────────────────────────────
fn gruvbox() -> Theme {
    Theme {
        bg:       Color::Rgb( 29,  32,  33),
        bg2:      Color::Rgb( 40,  40,  40),
        bg3:      Color::Rgb( 60,  56,  54),
        bg_sel:   Color::Rgb( 80,  73,  69),
        bright:   Color::Rgb(235, 219, 178),
        mid:      Color::Rgb(168, 153, 132),
        dim:      Color::Rgb(102,  92,  84),
        vdim:     Color::Rgb( 80,  73,  69),
        accent:   Color::Rgb(250, 189,  47),
        accent2:  Color::Rgb(215, 153,  33),
        accent3:  Color::Rgb(100,  70,  10),
        green:    Color::Rgb(184, 187,  38),
        blue:     Color::Rgb(131, 165, 152),
        red:      Color::Rgb(251,  73,  52),
        yellow:   Color::Rgb(250, 189,  47),
        purple:   Color::Rgb(211, 134, 155),
        cyan:     Color::Rgb(142, 192, 124),
        orange:   Color::Rgb(254, 128,  25),
        teal:     Color::Rgb(142, 192, 124),
        pink:     Color::Rgb(211, 134, 155),
        match_hl: Color::Rgb(254, 128,  25),
    }
}

// ─── Catppuccin Mocha ─────────────────────────────────────────────────────────
fn catppuccin() -> Theme {
    Theme {
        bg:       Color::Rgb( 30,  30,  46),
        bg2:      Color::Rgb( 24,  24,  37),
        bg3:      Color::Rgb( 49,  50,  68),
        bg_sel:   Color::Rgb( 69,  71,  90),
        bright:   Color::Rgb(205, 214, 244),
        mid:      Color::Rgb(166, 173, 200),
        dim:      Color::Rgb(108, 112, 134),
        vdim:     Color::Rgb( 69,  71,  90),
        accent:   Color::Rgb(245, 194, 231),
        accent2:  Color::Rgb(203, 166, 247),
        accent3:  Color::Rgb( 80,  50,  90),
        green:    Color::Rgb(166, 227, 161),
        blue:     Color::Rgb(137, 180, 250),
        red:      Color::Rgb(243, 139, 168),
        yellow:   Color::Rgb(249, 226, 175),
        purple:   Color::Rgb(203, 166, 247),
        cyan:     Color::Rgb(137, 220, 235),
        orange:   Color::Rgb(250, 179, 135),
        teal:     Color::Rgb(148, 226, 213),
        pink:     Color::Rgb(245, 194, 231),
        match_hl: Color::Rgb(249, 226, 175),
    }
}

// ─── Sky Blue ─────────────────────────────────────────────────────────────────
fn skyblue() -> Theme {
    Theme {
        bg:       Color::Rgb(  8,  18,  32),
        bg2:      Color::Rgb( 10,  24,  44),
        bg3:      Color::Rgb( 16,  36,  60),
        bg_sel:   Color::Rgb( 20,  48,  80),
        bright:   Color::Rgb(220, 240, 255),
        mid:      Color::Rgb(140, 190, 230),
        dim:      Color::Rgb( 60, 100, 150),
        vdim:     Color::Rgb( 25,  50,  85),
        accent:   Color::Rgb( 56, 189, 248),
        accent2:  Color::Rgb( 14, 165, 233),
        accent3:  Color::Rgb(  7,  89, 133),
        green:    Color::Rgb( 52, 211, 153),
        blue:     Color::Rgb( 96, 165, 250),
        red:      Color::Rgb(248, 113, 113),
        yellow:   Color::Rgb(250, 204,  21),
        purple:   Color::Rgb(167, 139, 250),
        cyan:     Color::Rgb(103, 232, 249),
        orange:   Color::Rgb(249, 115,  22),
        teal:     Color::Rgb( 45, 212, 191),
        pink:     Color::Rgb(244, 114, 182),
        match_hl: Color::Rgb( 56, 189, 248),
    }
}

// ─── Tokyo Night ──────────────────────────────────────────────────────────────
fn tokyonight() -> Theme {
    Theme {
        bg:       Color::Rgb( 26,  27,  38),
        bg2:      Color::Rgb( 16,  16,  28),
        bg3:      Color::Rgb( 36,  40,  59),
        bg_sel:   Color::Rgb( 41,  46,  66),
        bright:   Color::Rgb(192, 202, 245),
        mid:      Color::Rgb(122, 135, 180),
        dim:      Color::Rgb( 65,  72, 104),
        vdim:     Color::Rgb( 41,  46,  66),
        accent:   Color::Rgb(122, 162, 247),
        accent2:  Color::Rgb(187, 154, 247),
        accent3:  Color::Rgb( 52,  60, 100),
        green:    Color::Rgb(158, 206, 106),
        blue:     Color::Rgb(122, 162, 247),
        red:      Color::Rgb(247, 118, 142),
        yellow:   Color::Rgb(224, 175, 104),
        purple:   Color::Rgb(187, 154, 247),
        cyan:     Color::Rgb(125, 207, 255),
        orange:   Color::Rgb(255, 158,  84),
        teal:     Color::Rgb( 42, 195, 222),
        pink:     Color::Rgb(255, 117, 127),
        match_hl: Color::Rgb(224, 175, 104),
    }
}

// ─── Ayu Dark ─────────────────────────────────────────────────────────────────
fn ayu() -> Theme {
    Theme {
        bg:       Color::Rgb( 13,  17,  23),
        bg2:      Color::Rgb( 10,  14,  20),
        bg3:      Color::Rgb( 21,  28,  38),
        bg_sel:   Color::Rgb( 30,  42,  58),
        bright:   Color::Rgb(200, 213, 230),
        mid:      Color::Rgb(125, 150, 175),
        dim:      Color::Rgb( 55,  76, 100),
        vdim:     Color::Rgb( 25,  38,  55),
        accent:   Color::Rgb(229, 181,  69),
        accent2:  Color::Rgb(255, 140,  61),
        accent3:  Color::Rgb( 90,  60,  15),
        green:    Color::Rgb(149, 230, 203),
        blue:     Color::Rgb( 83, 154, 252),
        red:      Color::Rgb(255, 106, 106),
        yellow:   Color::Rgb(229, 181,  69),
        purple:   Color::Rgb(167, 130, 250),
        cyan:     Color::Rgb( 80, 213, 255),
        orange:   Color::Rgb(255, 140,  61),
        teal:     Color::Rgb(149, 230, 203),
        pink:     Color::Rgb(255, 128, 160),
        match_hl: Color::Rgb(229, 181,  69),
    }
}

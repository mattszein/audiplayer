use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug)]
pub struct TextStyle {
    pub small: Modifier,
    pub normal: Modifier,
    pub emphasis: Modifier,
}

impl TextStyle {
    pub fn default_style() -> Self {
        Self {
            small: Modifier::DIM,
            normal: Modifier::empty(),
            emphasis: Modifier::BOLD,
        }
    }
}

/// A palette defines colors for both dark and light modes.
/// Accent colors (primary, secondary, highlight, support) are per-mode
/// so light mode can use darker/more saturated variants for contrast.
struct Palette {
    dark_primary: Color,
    dark_secondary: Color,
    dark_highlight: Color,
    dark_support: Color,
    dark_default: Color,
    dark_middle: Color,
    dark_bg: Color,
    light_primary: Color,
    light_secondary: Color,
    light_highlight: Color,
    light_support: Color,
    light_default: Color,
    light_middle: Color,
    light_bg: Color,
}

/// The resolved theme used for rendering.
#[derive(Debug)]
pub struct Theme {
    pub name: &'static str,
    pub mode: ThemeMode,
    pub primary: Color,
    pub secondary: Color,
    pub default: Color,
    pub highlight: Color,
    pub middle: Color,
    pub support: Color,
    pub bg: Color,
    pub text: TextStyle,
}

impl Theme {
    fn from_palette(name: &'static str, p: &Palette, mode: ThemeMode) -> Self {
        let (primary, secondary, highlight, support, default, middle, bg) = match mode {
            ThemeMode::Dark => (
                p.dark_primary, p.dark_secondary, p.dark_highlight, p.dark_support,
                p.dark_default, p.dark_middle, p.dark_bg,
            ),
            ThemeMode::Light => (
                p.light_primary, p.light_secondary, p.light_highlight, p.light_support,
                p.light_default, p.light_middle, p.light_bg,
            ),
        };
        Self {
            name, mode, primary, secondary, default, highlight, middle, support, bg,
            text: TextStyle::default_style(),
        }
    }

    // ── Palettes ──────────────────────────────────────────────────

    fn default_palette() -> Palette {
        Palette {
            dark_primary: Color::Cyan,
            dark_secondary: Color::Yellow,
            dark_highlight: Color::Green,
            dark_support: Color::Blue,
            dark_default: Color::White,
            dark_middle: Color::DarkGray,
            dark_bg: Color::Reset,
            light_primary: Color::Rgb(0, 130, 150),
            light_secondary: Color::Rgb(160, 120, 0),
            light_highlight: Color::Rgb(0, 130, 50),
            light_support: Color::Rgb(30, 60, 180),
            light_default: Color::Black,
            light_middle: Color::Rgb(120, 120, 120),
            light_bg: Color::Rgb(235, 235, 235),
        }
    }

    fn gruvbox_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(214, 93, 14),      // orange
            dark_secondary: Color::Rgb(215, 153, 33),    // yellow
            dark_highlight: Color::Rgb(152, 151, 26),     // green
            dark_support: Color::Rgb(69, 133, 136),       // aqua
            dark_default: Color::Rgb(235, 219, 178),      // fg0
            dark_middle: Color::Rgb(146, 131, 116),       // gray
            dark_bg: Color::Rgb(40, 40, 40),              // bg0
            light_primary: Color::Rgb(175, 58, 3),        // orange dark
            light_secondary: Color::Rgb(181, 118, 20),    // yellow dark
            light_highlight: Color::Rgb(121, 116, 14),    // green dark
            light_support: Color::Rgb(42, 102, 107),      // aqua dark
            light_default: Color::Rgb(60, 56, 54),        // fg
            light_middle: Color::Rgb(146, 131, 116),      // gray
            light_bg: Color::Rgb(251, 241, 199),          // bg0
        }
    }

    fn tokyo_night_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(122, 162, 247),     // blue
            dark_secondary: Color::Rgb(224, 175, 104),    // yellow
            dark_highlight: Color::Rgb(158, 206, 106),    // green
            dark_support: Color::Rgb(187, 154, 247),      // purple
            dark_default: Color::Rgb(192, 202, 245),      // fg
            dark_middle: Color::Rgb(86, 95, 137),         // comment
            dark_bg: Color::Rgb(26, 27, 38),              // bg
            light_primary: Color::Rgb(52, 84, 190),       // blue day
            light_secondary: Color::Rgb(143, 100, 26),    // yellow day
            light_highlight: Color::Rgb(56, 120, 42),     // green day
            light_support: Color::Rgb(110, 68, 190),      // purple day
            light_default: Color::Rgb(52, 59, 88),        // fg day
            light_middle: Color::Rgb(120, 125, 150),      // comment day
            light_bg: Color::Rgb(212, 216, 232),          // bg day
        }
    }

    fn rose_pine_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(235, 188, 186),     // rose
            dark_secondary: Color::Rgb(246, 193, 119),    // gold
            dark_highlight: Color::Rgb(156, 207, 216),    // foam
            dark_support: Color::Rgb(196, 167, 231),      // iris
            dark_default: Color::Rgb(224, 222, 244),      // text
            dark_middle: Color::Rgb(110, 106, 134),       // muted
            dark_bg: Color::Rgb(25, 23, 36),              // base
            light_primary: Color::Rgb(180, 99, 122),      // rose dawn
            light_secondary: Color::Rgb(174, 131, 55),    // gold dawn
            light_highlight: Color::Rgb(40, 105, 131),    // foam dawn
            light_support: Color::Rgb(127, 100, 180),     // iris dawn
            light_default: Color::Rgb(87, 82, 121),       // text dawn
            light_middle: Color::Rgb(121, 117, 147),      // muted dawn
            light_bg: Color::Rgb(250, 244, 237),          // base dawn
        }
    }

    fn catppuccin_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(137, 180, 250),     // blue mocha
            dark_secondary: Color::Rgb(249, 226, 175),    // yellow
            dark_highlight: Color::Rgb(166, 227, 161),    // green
            dark_support: Color::Rgb(203, 166, 247),      // mauve
            dark_default: Color::Rgb(205, 214, 244),      // text mocha
            dark_middle: Color::Rgb(108, 112, 134),       // overlay0
            dark_bg: Color::Rgb(30, 30, 46),              // base mocha
            light_primary: Color::Rgb(30, 102, 245),      // blue latte
            light_secondary: Color::Rgb(223, 142, 29),    // yellow latte
            light_highlight: Color::Rgb(64, 160, 43),     // green latte
            light_support: Color::Rgb(136, 57, 239),      // mauve latte
            light_default: Color::Rgb(76, 79, 105),       // text latte
            light_middle: Color::Rgb(124, 127, 147),      // overlay0 latte
            light_bg: Color::Rgb(239, 241, 245),          // base latte
        }
    }

    fn everforest_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(163, 190, 140),     // green
            dark_secondary: Color::Rgb(219, 188, 127),    // yellow
            dark_highlight: Color::Rgb(131, 194, 159),    // aqua
            dark_support: Color::Rgb(214, 153, 182),      // purple
            dark_default: Color::Rgb(211, 198, 170),      // fg
            dark_middle: Color::Rgb(133, 146, 137),       // gray1
            dark_bg: Color::Rgb(39, 51, 43),              // bg0
            light_primary: Color::Rgb(93, 137, 82),       // green light
            light_secondary: Color::Rgb(170, 130, 48),    // yellow light
            light_highlight: Color::Rgb(53, 140, 91),     // aqua light
            light_support: Color::Rgb(160, 90, 132),      // purple light
            light_default: Color::Rgb(92, 107, 99),       // fg
            light_middle: Color::Rgb(130, 140, 125),      // gray
            light_bg: Color::Rgb(253, 246, 227),          // bg0
        }
    }

    fn kanagawa_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(126, 156, 216),     // crystalBlue
            dark_secondary: Color::Rgb(226, 194, 130),    // carpYellow
            dark_highlight: Color::Rgb(152, 187, 108),    // springGreen
            dark_support: Color::Rgb(210, 126, 153),      // sakuraPink
            dark_default: Color::Rgb(220, 215, 186),      // fujiWhite
            dark_middle: Color::Rgb(114, 113, 105),       // fujiGray
            dark_bg: Color::Rgb(31, 31, 40),              // sumiInk1
            light_primary: Color::Rgb(73, 109, 176),      // blue lotus
            light_secondary: Color::Rgb(171, 132, 55),    // yellow lotus
            light_highlight: Color::Rgb(98, 138, 56),     // green lotus
            light_support: Color::Rgb(176, 82, 121),      // pink lotus
            light_default: Color::Rgb(84, 84, 109),       // fg
            light_middle: Color::Rgb(127, 127, 143),      // gray
            light_bg: Color::Rgb(242, 236, 228),          // lotusWhite3
        }
    }

    fn nord_palette() -> Palette {
        Palette {
            dark_primary: Color::Rgb(136, 192, 208),     // nord8
            dark_secondary: Color::Rgb(235, 203, 139),    // nord13
            dark_highlight: Color::Rgb(163, 190, 140),    // nord14
            dark_support: Color::Rgb(180, 142, 173),      // nord15
            dark_default: Color::Rgb(216, 222, 233),      // nord4
            dark_middle: Color::Rgb(97, 110, 136),        // nord3 bright
            dark_bg: Color::Rgb(46, 52, 64),              // nord0
            light_primary: Color::Rgb(53, 129, 153),      // frost dark
            light_secondary: Color::Rgb(163, 130, 50),    // yellow dark
            light_highlight: Color::Rgb(93, 131, 80),     // green dark
            light_support: Color::Rgb(132, 90, 125),      // purple dark
            light_default: Color::Rgb(59, 66, 82),        // nord1
            light_middle: Color::Rgb(110, 120, 140),      // muted
            light_bg: Color::Rgb(236, 239, 244),          // nord6
        }
    }

    fn magenta_palette() -> Palette {
        Palette {
            dark_primary: Color::Magenta,
            dark_secondary: Color::LightMagenta,
            dark_highlight: Color::Green,
            dark_support: Color::Rgb(100, 100, 180),
            dark_default: Color::White,
            dark_middle: Color::DarkGray,
            dark_bg: Color::Rgb(32, 20, 32),
            light_primary: Color::Rgb(150, 0, 150),
            light_secondary: Color::Rgb(170, 50, 130),
            light_highlight: Color::Rgb(0, 130, 50),
            light_support: Color::Rgb(70, 50, 150),
            light_default: Color::Black,
            light_middle: Color::Rgb(120, 110, 120),
            light_bg: Color::Rgb(240, 230, 240),
        }
    }

    // ── Constructors ──────────────────────────────────────────────

    pub fn default_theme() -> Self {
        Self::from_palette("default", &Self::default_palette(), ThemeMode::Dark)
    }

    pub fn preset_names() -> &'static [&'static str] {
        &[
            "default",
            "gruvbox",
            "tokyo-night",
            "rose-pine",
            "catppuccin",
            "everforest",
            "kanagawa",
            "nord",
            "magenta",
        ]
    }

    pub fn from_name(name: &str, mode: ThemeMode) -> Self {
        match name {
            "gruvbox" => Self::from_palette("gruvbox", &Self::gruvbox_palette(), mode),
            "tokyo-night" => Self::from_palette("tokyo-night", &Self::tokyo_night_palette(), mode),
            "rose-pine" => Self::from_palette("rose-pine", &Self::rose_pine_palette(), mode),
            "catppuccin" => Self::from_palette("catppuccin", &Self::catppuccin_palette(), mode),
            "everforest" => Self::from_palette("everforest", &Self::everforest_palette(), mode),
            "kanagawa" => Self::from_palette("kanagawa", &Self::kanagawa_palette(), mode),
            "nord" => Self::from_palette("nord", &Self::nord_palette(), mode),
            "magenta" => Self::from_palette("magenta", &Self::magenta_palette(), mode),
            _ => Self::from_palette("default", &Self::default_palette(), mode),
        }
    }

    pub fn with_mode(&self, mode: ThemeMode) -> Self {
        Self::from_name(self.name, mode)
    }

    pub fn cycle_preset(&self) -> Self {
        let names = Self::preset_names();
        let idx = names.iter().position(|&n| n == self.name).unwrap_or(0);
        let next = (idx + 1) % names.len();
        Self::from_name(names[next], self.mode)
    }

    // ── Convenience style methods ─────────────────────────────────

    pub fn base(&self) -> Style {
        Style::default().fg(self.default).bg(self.bg)
    }

    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.primary)
    }

    pub fn border_inactive(&self) -> Style {
        Style::default().fg(self.middle)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(self.text.emphasis)
    }

    pub fn selected_bg(&self) -> Style {
        match self.mode {
            ThemeMode::Dark => Style::default().bg(self.middle),
            ThemeMode::Light => Style::default().bg(Color::Rgb(210, 210, 210)),
        }
    }

    pub fn badge(&self) -> Style {
        Style::default()
            .bg(self.primary)
            .fg(match self.mode {
                ThemeMode::Dark => Color::Black,
                ThemeMode::Light => Color::White,
            })
            .add_modifier(self.text.emphasis)
    }

    pub fn header(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(self.text.emphasis)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.middle)
    }

    pub fn playing(&self) -> Style {
        Style::default()
            .fg(self.highlight)
            .add_modifier(self.text.emphasis)
    }
}

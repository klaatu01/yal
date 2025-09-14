use yal_core::Theme;

#[derive(Copy, Clone, Debug)]
pub struct ThemeRef {
    pub name: &'static str,
    pub bg_color: &'static str,
    pub fg_color: &'static str,
    pub bg_font_color: &'static str,
    pub fg_font_color: &'static str,
}

impl ThemeRef {
    pub const fn new(
        name: &'static str,
        bg_color: &'static str,
        fg_color: &'static str,
        bg_font_color: &'static str,
        fg_font_color: &'static str,
    ) -> Self {
        Self {
            name,
            bg_color,
            fg_color,
            bg_font_color,
            fg_font_color,
        }
    }

    pub fn to_owned(self) -> Theme {
        Theme {
            name: Some(self.name.to_string()),
            bg_color: Some(self.bg_color.to_string()),
            fg_color: Some(self.fg_color.to_string()),
            bg_font_color: Some(self.bg_font_color.to_string()),
            fg_font_color: Some(self.fg_font_color.to_string()),
        }
    }
}

impl From<ThemeRef> for Theme {
    fn from(t: ThemeRef) -> Self {
        t.to_owned()
    }
}

/* ------------------------------ Theme constants ------------------------------ */
/* Notes:
 * - `fg_color` is treated as the *highlight background*.
 * - `bg_font_color` is normal text on `bg_color`.
 * - `fg_font_color` is text on the highlighted row (`fg_color`).
 * Tweak to taste per your UI.
 */

// Monokai (classic)
pub const MONOKAI: ThemeRef = ThemeRef::new(
    "monokai", "#272822", // bg
    "#49483E", // highlight bg
    "#F8F8F2", // text on bg
    "#FFFFFF", // text on highlight
);

// Dracula
pub const DRACULA: ThemeRef = ThemeRef::new("dracula", "#282A36", "#44475A", "#F8F8F2", "#FFFFFF");

// Catppuccin — Latte (light)
pub const CATPPUCCIN_LATTE: ThemeRef = ThemeRef::new(
    "catppuccin-latte",
    "#EFF1F5", // base
    "#CCD0DA", // surface2-ish as highlight
    "#4C4F69", // text
    "#1E1E2E", // strong text on highlight
);

// Catppuccin — Frappé (dark)
pub const CATPPUCCIN_FRAPPE: ThemeRef = ThemeRef::new(
    "catppuccin-frappe",
    "#303446", // base
    "#414559", // surface1
    "#C6D0F5", // text
    "#E6E9EF", // text on highlight
);

// Catppuccin — Macchiato (dark)
pub const CATPPUCCIN_MACCHIATO: ThemeRef = ThemeRef::new(
    "catppuccin-macchiato",
    "#24273A",
    "#363A4F",
    "#CAD3F5",
    "#E8E8E8",
);

// Catppuccin — Mocha (dark)
pub const CATPPUCCIN_MOCHA: ThemeRef = ThemeRef::new(
    "catppuccin-mocha",
    "#1E1E2E",
    "#313244",
    "#CDD6F4",
    "#E6E6E6",
);

// Solarized Dark
pub const SOLARIZED_DARK: ThemeRef =
    ThemeRef::new("solarized-dark", "#002B36", "#073642", "#93A1A1", "#FDF6E3");

// Solarized Light
pub const SOLARIZED_LIGHT: ThemeRef = ThemeRef::new(
    "solarized-light",
    "#FDF6E3",
    "#EEE8D5",
    "#586E75",
    "#073642",
);

// Gruvbox Dark (hard-ish)
pub const GRUVBOX_DARK: ThemeRef =
    ThemeRef::new("gruvbox-dark", "#282828", "#3C3836", "#EBDBB2", "#FBF1C7");

// Gruvbox Light
pub const GRUVBOX_LIGHT: ThemeRef =
    ThemeRef::new("gruvbox-light", "#FBF1C7", "#EBDBB2", "#3C3836", "#282828");

// Nord
pub const NORD: ThemeRef = ThemeRef::new("nord", "#2E3440", "#3B4252", "#D8DEE9", "#ECEFF4");

// One Dark
pub const ONE_DARK: ThemeRef =
    ThemeRef::new("one-dark", "#282C34", "#3E4451", "#ABB2BF", "#FFFFFF");

// Tokyo Night (classic)
pub const TOKYO_NIGHT: ThemeRef =
    ThemeRef::new("tokyo-night", "#1A1B26", "#2F334D", "#C0CAF5", "#E6E6E6");

// Tokyo Night Storm
pub const TOKYO_NIGHT_STORM: ThemeRef = ThemeRef::new(
    "tokyo-night-storm",
    "#24283B",
    "#283457",
    "#C0CAF5",
    "#E6E6E6",
);

/* --------------------------------- Registry --------------------------------- */

pub const ALL: &[ThemeRef] = &[
    MONOKAI,
    DRACULA,
    CATPPUCCIN_LATTE,
    CATPPUCCIN_FRAPPE,
    CATPPUCCIN_MACCHIATO,
    CATPPUCCIN_MOCHA,
    SOLARIZED_DARK,
    SOLARIZED_LIGHT,
    GRUVBOX_DARK,
    GRUVBOX_LIGHT,
    NORD,
    ONE_DARK,
    TOKYO_NIGHT,
    TOKYO_NIGHT_STORM,
];

/// Case-insensitive lookup. Supports a few aliases.
pub fn by_name(name: &str) -> Option<ThemeRef> {
    let n = name.trim().to_lowercase();
    let normalized = match n.as_str() {
        // aliases
        "one" | "onedark" | "one-dark-pro" => "one-dark",
        "tokyo" | "tokyonight" => "tokyo-night",
        "tokyo-storm" | "tokyonight-storm" => "tokyo-night-storm",
        "catppuccin" => "catppuccin-mocha", // default flavor
        "catppuccin-latte" | "latte" => "catppuccin-latte",
        "catppuccin-frappe" | "frappe" => "catppuccin-frappe",
        "catppuccin-macchiato" | "macchiato" => "catppuccin-macchiato",
        "catppuccin-mocha" | "mocha" => "catppuccin-mocha",
        other => other,
    };

    ALL.iter().copied().find(|t| t.name == normalized)
}

/// Owned theme list (useful for config UIs).
pub fn list_owned() -> Vec<Theme> {
    ALL.iter().copied().map(Theme::from).collect()
}

/* ----------------------------------- Tests ---------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ref_to_owned() {
        let t: Theme = MONOKAI.into();
        assert_eq!(t.name.as_deref(), Some("monokai"));
        assert_eq!(t.bg_color.as_deref(), Some("#272822"));
        assert_eq!(t.fg_color.as_deref(), Some("#49483E"));
    }

    #[test]
    fn lookup_aliases() {
        assert_eq!(by_name("OneDark").unwrap().name, "one-dark");
        assert_eq!(by_name("catppuccin").unwrap().name, "catppuccin-mocha");
        assert_eq!(
            by_name("tokyonight-storm").unwrap().name,
            "tokyo-night-storm"
        );
    }
}

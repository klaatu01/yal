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

pub const MONOKAI: ThemeRef = ThemeRef::new("monokai", "#272822", "#49483E", "#F8F8F2", "#FFFFFF");

pub const DRACULA: ThemeRef = ThemeRef::new("dracula", "#282A36", "#44475A", "#F8F8F2", "#FFFFFF");

pub const CATPPUCCIN_LATTE: ThemeRef = ThemeRef::new(
    "catppuccin-latte",
    "#EFF1F5",
    "#CCD0DA",
    "#4C4F69",
    "#1E1E2E",
);

pub const CATPPUCCIN_FRAPPE: ThemeRef = ThemeRef::new(
    "catppuccin-frappe",
    "#303446",
    "#414559",
    "#C6D0F5",
    "#E6E9EF",
);

pub const CATPPUCCIN_MACCHIATO: ThemeRef = ThemeRef::new(
    "catppuccin-macchiato",
    "#24273A",
    "#363A4F",
    "#CAD3F5",
    "#E8E8E8",
);

pub const CATPPUCCIN_MOCHA: ThemeRef = ThemeRef::new(
    "catppuccin-mocha",
    "#1E1E2E",
    "#313244",
    "#CDD6F4",
    "#E6E6E6",
);

pub const SOLARIZED_DARK: ThemeRef =
    ThemeRef::new("solarized-dark", "#002B36", "#073642", "#93A1A1", "#FDF6E3");

pub const SOLARIZED_LIGHT: ThemeRef = ThemeRef::new(
    "solarized-light",
    "#FDF6E3",
    "#EEE8D5",
    "#586E75",
    "#073642",
);

pub const GRUVBOX_DARK: ThemeRef =
    ThemeRef::new("gruvbox-dark", "#282828", "#3C3836", "#EBDBB2", "#FBF1C7");

pub const GRUVBOX_LIGHT: ThemeRef =
    ThemeRef::new("gruvbox-light", "#FBF1C7", "#EBDBB2", "#3C3836", "#282828");

pub const NORD: ThemeRef = ThemeRef::new("nord", "#2E3440", "#3B4252", "#D8DEE9", "#ECEFF4");

pub const ONE_DARK: ThemeRef =
    ThemeRef::new("one-dark", "#282C34", "#3E4451", "#ABB2BF", "#FFFFFF");

pub const TOKYO_NIGHT: ThemeRef =
    ThemeRef::new("tokyo-night", "#1A1B26", "#2F334D", "#C0CAF5", "#E6E6E6");

pub const TOKYO_NIGHT_STORM: ThemeRef = ThemeRef::new(
    "tokyo-night-storm",
    "#24283B",
    "#283457",
    "#C0CAF5",
    "#E6E6E6",
);

pub const YAL_RED: ThemeRef = ThemeRef::new("yal-red", "#24273A", "#FF7A93", "#CAD3F5", "#242424");

pub const YAL_BLUE: ThemeRef =
    ThemeRef::new("yal-blue", "#24273A", "#7AA2F7", "#CAD3F5", "#242424");

pub const YAL_GREEN: ThemeRef =
    ThemeRef::new("yal-green", "#24273A", "#9ECE6A", "#CAD3F5", "#242424");

pub const YAL_YELLOW: ThemeRef =
    ThemeRef::new("yal-yellow", "#24273A", "#E0AF68", "#CAD3F5", "#242424");

pub const YAL_PURPLE: ThemeRef =
    ThemeRef::new("yal-purple", "#24273A", "#BB9AF7", "#CAD3F5", "#242424");

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
    YAL_RED,
    YAL_BLUE,
    YAL_GREEN,
    YAL_YELLOW,
    YAL_PURPLE,
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
        "yalred" | "yal-red" => "yal-red",
        "yalblue" | "yal-blue" => "yal-blue",
        "yalgreen" | "yal-green" => "yal-green",
        "yalyellow" | "yal-yellow" => "yal-yellow",
        "yalpurple" | "yal-purple" => "yal-purple",
        other => other,
    };

    ALL.iter().copied().find(|t| t.name == normalized)
}

pub fn list_owned() -> Vec<Theme> {
    ALL.iter().copied().map(Theme::from).collect()
}

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

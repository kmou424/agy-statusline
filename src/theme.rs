use std::{fs, path::Path};

use serde::Deserialize;

use crate::home_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRole {
    Accent,
    MdLink,
    Text,
    Muted,
    Dim,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorScheme {
    #[default]
    Terminal,
    Light,
    SolarizedLight,
    ColorblindFriendlyLight,
    Dark,
    SolarizedDark,
    ColorblindFriendlyDark,
    TokyoNight,
}

impl ColorScheme {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "solarized light" => Self::SolarizedLight,
            "colorblind-friendly light" => Self::ColorblindFriendlyLight,
            "dark" => Self::Dark,
            "solarized dark" => Self::SolarizedDark,
            "colorblind-friendly dark" => Self::ColorblindFriendlyDark,
            "tokyo night" => Self::TokyoNight,
            _ => Self::Terminal,
        }
    }
}

pub fn load_agy_color_scheme_from_path(path: &Path) -> ColorScheme {
    #[derive(Deserialize)]
    struct Settings {
        #[serde(rename = "colorScheme")]
        color_scheme: String,
    }
    if !fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return ColorScheme::Terminal;
    }
    fs::read_to_string(path)
        .ok()
        .filter(|data| data.trim_start().starts_with('{'))
        .and_then(|data| serde_json::from_str::<Settings>(&data).ok())
        .map_or(ColorScheme::Terminal, |settings| {
            ColorScheme::parse(&settings.color_scheme)
        })
}

pub fn load_agy_color_scheme() -> ColorScheme {
    home_dir().map_or(ColorScheme::Terminal, |home| {
        load_agy_color_scheme_from_path(
            &Path::new(&home).join(".gemini/antigravity-cli/settings.json"),
        )
    })
}

pub struct Palette {
    prefixes: [&'static str; 8],
}

macro_rules! rgb {
    ($(($r:literal, $g:literal, $b:literal)),+ $(,)?) => {
        [$(concat!("\x1b[38;2;", $r, ";", $g, ";", $b, "m")),+]
    };
}

impl Palette {
    pub fn for_scheme(scheme: ColorScheme) -> Self {
        let prefixes = match scheme {
            ColorScheme::Terminal => [
                "\x1b[96m", "\x1b[94m", "\x1b[39m", "\x1b[90m", "\x1b[2m", "\x1b[32m", "\x1b[33m",
                "\x1b[31m",
            ],
            ColorScheme::Light => rgb![
                (0, 105, 113),
                (0, 77, 153),
                (31, 35, 41),
                (78, 87, 98),
                (100, 107, 119),
                (26, 100, 47),
                (129, 76, 0),
                (166, 28, 46),
            ],
            ColorScheme::Dark => rgb![
                (94, 234, 212),
                (135, 183, 255),
                (230, 237, 243),
                (159, 173, 189),
                (120, 136, 156),
                (126, 231, 135),
                (242, 204, 96),
                (255, 123, 114),
            ],
            ColorScheme::SolarizedLight => rgb![
                (0, 108, 106),
                (24, 98, 154),
                (7, 54, 66),
                (71, 94, 101),
                (92, 111, 113),
                (74, 102, 0),
                (131, 87, 0),
                (166, 40, 42),
            ],
            ColorScheme::SolarizedDark => rgb![
                (64, 189, 178),
                (91, 170, 225),
                (238, 232, 213),
                (147, 161, 161),
                (111, 137, 142),
                (160, 178, 39),
                (218, 174, 66),
                (247, 116, 110),
            ],
            ColorScheme::TokyoNight => rgb![
                (125, 207, 255),
                (122, 162, 247),
                (192, 202, 245),
                (154, 165, 206),
                (122, 130, 163),
                (158, 206, 106),
                (224, 175, 104),
                (247, 118, 142),
            ],
            ColorScheme::ColorblindFriendlyLight => rgb![
                (0, 105, 133),
                (0, 74, 126),
                (30, 35, 42),
                (78, 88, 101),
                (99, 108, 120),
                (0, 98, 158),
                (133, 86, 0),
                (161, 57, 0),
            ],
            ColorScheme::ColorblindFriendlyDark => rgb![
                (86, 180, 233),
                (199, 167, 233),
                (235, 240, 246),
                (161, 174, 190),
                (123, 140, 159),
                (70, 159, 222),
                (230, 159, 0),
                (240, 111, 56),
            ],
        };
        Self { prefixes }
    }

    pub fn fg(&self, role: ColorRole, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let prefix = self.prefixes[role as usize];
        format!("{prefix}{text}\x1b[0m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schema_names_and_styles_semantically() {
        for (raw, scheme) in [
            ("dark", ColorScheme::Dark),
            (" \tLight ", ColorScheme::Light),
            ("sOLARIZED dARK", ColorScheme::SolarizedDark),
            ("Solarized Light", ColorScheme::SolarizedLight),
            (
                "colorblind-friendly dark",
                ColorScheme::ColorblindFriendlyDark,
            ),
            (
                "colorblind-friendly light",
                ColorScheme::ColorblindFriendlyLight,
            ),
            ("Tokyo Night", ColorScheme::TokyoNight),
        ] {
            assert_eq!(ColorScheme::parse(raw), scheme);
            let palette = Palette::for_scheme(scheme);
            let success = palette.fg(ColorRole::Success, "test");
            assert!(success.starts_with("\x1b[38;2;"));
            assert!(success.ends_with("mtest\x1b[0m"));
            assert_ne!(success, palette.fg(ColorRole::Error, "test"));
        }
        for raw in ["terminal", "", "Solarized", "auto"] {
            assert_eq!(ColorScheme::parse(raw), ColorScheme::Terminal);
        }
        let terminal = Palette::for_scheme(ColorScheme::Terminal);
        assert_eq!(
            terminal.fg(ColorRole::Accent, "test"),
            "\x1b[96mtest\x1b[0m"
        );
        assert!(terminal.fg(ColorRole::Dim, "").is_empty());
    }
}

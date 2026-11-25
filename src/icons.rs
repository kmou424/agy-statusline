#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconMode {
    #[default]
    Auto,
    Nerd,
    Ascii,
}

impl IconMode {
    pub fn resolve(self) -> ResolvedIconMode {
        match self {
            Self::Nerd => ResolvedIconMode::Nerd,
            Self::Auto if detect_nerd_font() => ResolvedIconMode::Nerd,
            Self::Auto | Self::Ascii => ResolvedIconMode::Ascii,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedIconMode {
    Nerd,
    Ascii,
}

pub fn detect_nerd_font_with<F: Fn(&str) -> Option<String>>(get_env: F) -> bool {
    const PROGRAMS: &[&str] = &[
        "iterm.app",
        "ghostty",
        "wezterm",
        "kitty",
        "rio",
        "tabby",
        "windowsterminal",
        "vscode",
        "herdr",
    ];
    get_env("HERDR_ENV").as_deref() == Some("1")
        || get_env("TERM").as_deref() == Some("xterm-kitty")
        || get_env("WT_SESSION").is_some_and(|session| !session.is_empty())
        || ["TERM_PROGRAM", "LC_TERMINAL"].into_iter().any(|key| {
            get_env(key).is_some_and(|program| PROGRAMS.contains(&program.to_lowercase().as_str()))
        })
}

pub fn detect_nerd_font() -> bool {
    detect_nerd_font_with(|key| std::env::var(key).ok())
}

pub struct Glyphs {
    pub cwd: &'static str,
    pub session: &'static str,
    pub git: &'static str,
    pub context: &'static str,
    pub model: &'static str,
    pub fast: &'static str,
    pub do_not_pause: &'static str,
    pub stall: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub cache_hit: &'static str,
    pub cost: &'static str,
    pub ahead: &'static str,
    pub behind: &'static str,
    pub diverged: &'static str,
    pub conflicted: &'static str,
    pub stashed: &'static str,
    pub modified: &'static str,
    pub staged: &'static str,
    pub untracked: &'static str,
    pub renamed: &'static str,
    pub deleted: &'static str,
    pub bar_filled: &'static str,
    pub bar_empty: &'static str,
}

impl Glyphs {
    pub fn for_mode(mode: ResolvedIconMode) -> &'static Self {
        match mode {
            ResolvedIconMode::Nerd => &Self {
                cwd: "\u{F07C}",
                session: "\u{F02B}",
                git: "\u{F418}",
                context: "\u{F0C9}",
                model: "\u{F121}",
                fast: "\u{F0E7}",
                do_not_pause: "\u{F04B}",
                stall: "\u{F071}",
                input: "\u{F093}",
                output: "\u{F019}",
                cache_hit: "\u{F1C0}",
                cost: "\u{F155}",
                ahead: "↑",
                behind: "↓",
                diverged: "⇕",
                conflicted: "=",
                stashed: "$",
                modified: "!",
                staged: "+",
                untracked: "?",
                renamed: "»",
                deleted: "✘",
                bar_filled: "▰",
                bar_empty: "▱",
            },
            ResolvedIconMode::Ascii => &Self {
                cwd: "@",
                session: "s",
                git: "*",
                context: "#",
                model: "M",
                fast: ">",
                do_not_pause: "play",
                stall: "!",
                input: "↑",
                output: "↓",
                cache_hit: "CH",
                cost: "$",
                ahead: "^",
                behind: "v",
                diverged: "^v",
                conflicted: "=",
                stashed: "S",
                modified: "!",
                staged: "A",
                untracked: "?",
                renamed: "r",
                deleted: "x",
                bar_filled: "#",
                bar_empty: "-",
            },
        }
    }
}

pub fn runtime_symbol(name: &str, mode: ResolvedIconMode) -> &str {
    if mode == ResolvedIconMode::Ascii {
        return name;
    }
    match name {
        "nodejs" => "\u{E718}",
        "rust" => "\u{E7A8}",
        "go" => "\u{E626}",
        "python" => "\u{E73C}",
        "ruby" => "\u{E739}",
        "java" => "\u{E256}",
        "cpp" => "\u{E61D}",
        "c" => "\u{E61E}",
        "swift" => "\u{E755}",
        "kotlin" => "\u{E634}",
        "deno" => "\u{E7FB}",
        "bun" => "\u{E6FB}",
        "php" => "\u{E73D}",
        "lua" => "\u{E620}",
        "zig" => "\u{E6A9}",
        _ => "\u{F120}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_terminal_environment() {
        for (key, value) in [
            ("HERDR_ENV", "1"),
            ("TERM", "xterm-kitty"),
            ("WT_SESSION", " "),
            ("TERM_PROGRAM", "iTerm.app"),
            ("TERM_PROGRAM", "WindowsTerminal"),
            ("TERM_PROGRAM", "vscode"),
            ("TERM_PROGRAM", "Rio"),
            ("LC_TERMINAL", "GHOSTTY"),
            ("LC_TERMINAL", "WezTerm"),
            ("LC_TERMINAL", "kitty"),
            ("LC_TERMINAL", "Tabby"),
            ("LC_TERMINAL", "herdr"),
        ] {
            assert!(detect_nerd_font_with(|k| (k == key).then(|| value.into())));
        }
        for (key, value) in [
            ("HERDR_ENV", "true"),
            ("TERM", "XTERM-KITTY"),
            ("WT_SESSION", ""),
            ("TERM_PROGRAM", " kitty "),
        ] {
            assert!(!detect_nerd_font_with(|k| (k == key).then(|| value.into())));
        }
        assert!(!detect_nerd_font_with(|_| None));
    }

    #[test]
    fn resolves_runtime_glyphs() {
        assert_eq!(runtime_symbol("rust", ResolvedIconMode::Nerd), "\u{E7A8}");
        assert_eq!(runtime_symbol("dotnet", ResolvedIconMode::Nerd), "\u{F120}");
        assert_eq!(runtime_symbol("dotnet", ResolvedIconMode::Ascii), "dotnet");
    }
}

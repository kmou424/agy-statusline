use std::path::Path;

use crate::{
    icons::{Glyphs, ResolvedIconMode, runtime_symbol},
    probes::{GitBranch, GitStatus, RuntimeInfo},
    text::{clean, format_count, truncate_to_width, visible_width},
    theme::{ColorRole::*, Palette},
};

pub const MAX_TERMINAL_WIDTH: usize = 4096;

pub struct FooterOptions {
    pub cwd: bool,
    pub conversation_title: bool,
    pub git_branch: bool,
    pub git_status: bool,
    pub runtime: bool,
    pub context: bool,
    pub tokens: bool,
    pub cost: bool,
}

impl Default for FooterOptions {
    fn default() -> Self {
        Self {
            cwd: true,
            conversation_title: false,
            git_branch: true,
            git_status: true,
            runtime: true,
            context: true,
            tokens: true,
            cost: true,
        }
    }
}

#[derive(Default)]
pub struct FooterData {
    pub cwd: Option<String>,
    pub home_dir: Option<String>,
    pub conversation_title: Option<String>,
    pub branch: Option<GitBranch>,
    pub git: GitStatus,
    pub runtime: Option<RuntimeInfo>,
    pub context_window_size: u64,
    pub used_percentage: Option<f64>,
    pub model_name: Option<String>,
    pub model_effort: Option<String>,
    pub cycle_mode: Option<String>,
    pub sandbox_enabled: bool,
    pub tool_confirmation_pending: bool,
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub current_turn_cache_hit_rate: Option<f64>,
    pub cost_usd: Option<f64>,
}

struct Part {
    text: String,
    compact: Option<String>,
    priority: u8,
}

impl Part {
    fn new(text: String, priority: u8) -> Self {
        Self {
            text,
            compact: None,
            priority,
        }
    }
}

fn cleaned(text: Option<&str>) -> Option<String> {
    text.map(clean).filter(|text| !text.is_empty())
}

fn align(left: &str, right: &str, width: usize) -> String {
    if right.is_empty() {
        return truncate_to_width(left, width);
    }
    let right_width = visible_width(right);
    if right_width >= width {
        return truncate_to_width(right, width);
    }
    let left = truncate_to_width(left, width - right_width - usize::from(!left.is_empty()));
    let gap = width - right_width - visible_width(&left);
    format!("{left}{:gap$}{right}", "")
}

fn cwd_part(raw: &str, home: Option<&str>, g: &Glyphs, palette: &Palette) -> Option<Part> {
    let cleaned_raw = clean(raw);
    if cleaned_raw.is_empty() {
        return None;
    }
    let text = home
        .filter(|home| !home.is_empty())
        .and_then(|home| Path::new(&cleaned_raw).strip_prefix(home).ok())
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                "~".to_owned()
            } else {
                format!("~/{}", relative.display())
            }
        })
        .unwrap_or(cleaned_raw);
    let compact = Path::new(&text)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&text);
    let glyph = palette.fg(MdLink, g.cwd);
    Some(Part {
        text: format!("{glyph} {}", palette.fg(Accent, &text)),
        compact: Some(format!("{glyph} {}", palette.fg(Accent, compact))),
        priority: 0,
    })
}

fn git_group(data: &FooterData, options: &FooterOptions, g: &Glyphs, palette: &Palette) -> String {
    let mut group = Vec::new();
    if options.git_branch {
        match &data.branch {
            Some(GitBranch::Detached) => {
                group.push(palette.fg(Warning, &format!("{} HEAD", g.git)));
            }
            Some(GitBranch::Named(branch)) => {
                if let Some(label) = cleaned(Some(branch)) {
                    group.push(palette.fg(
                        MdLink,
                        &format!("{} {}", g.git, truncate_to_width(&label, 20)),
                    ));
                }
            }
            None => {}
        }
    }
    if options.git_status {
        let git = &data.git;
        let mut counts: Vec<_> = [
            (git.conflicted, g.conflicted, Error),
            (git.deleted, g.deleted, Error),
            (git.modified, g.modified, Warning),
            (git.renamed, g.renamed, Warning),
            (git.staged, g.staged, Success),
            (git.untracked, g.untracked, Muted),
            (git.stashed, g.stashed, Muted),
        ]
        .into_iter()
        .filter(|&(n, _, _)| n > 0)
        .map(|(n, glyph, role)| palette.fg(role, &format!("{glyph}{n}")))
        .collect();
        match (git.ahead, git.behind) {
            (0, 0) => {}
            (ahead, 0) => counts.push(palette.fg(Success, &format!("{}{ahead}", g.ahead))),
            (0, behind) => counts.push(palette.fg(Warning, &format!("{}{behind}", g.behind))),
            (ahead, behind) => {
                counts.push(palette.fg(Warning, &format!("{}{ahead}/{behind}", g.diverged)))
            }
        }
        if !counts.is_empty() {
            group.push(format!(
                "{}{}{}",
                palette.fg(Dim, "["),
                counts.join(" "),
                palette.fg(Dim, "]")
            ));
        }
    }
    group.join(" ")
}

fn percentage(raw: Option<f64>) -> Option<f64> {
    raw.filter(|pct| pct.is_finite())
        .map(|pct| pct.clamp(0.0, 100.0))
}

fn context(
    data: &FooterData,
    g: &Glyphs,
    palette: &Palette,
    min_left: usize,
    width: usize,
) -> String {
    let pct = percentage(data.used_percentage);
    let n = pct.unwrap_or(0.0);
    let stress = if n >= 90.0 {
        Error
    } else if n >= 70.0 {
        Warning
    } else {
        Accent
    };
    let bar_stress = if n >= 80.0 {
        Error
    } else if n >= 50.0 {
        Warning
    } else {
        Success
    };
    let filled = ((n * 12.0) / 100.0).round().clamp(0.0, 12.0) as usize;
    let capacity = data.context_window_size;
    let used = pct.map_or_else(
        || "?".to_owned(),
        |n| format_count(((n * capacity as f64) / 100.0).round().max(0.0) as u64),
    );
    let icon = palette.fg(stress, g.context);
    let pct_styled = palette.fg(
        stress,
        &pct.map_or_else(|| "?".into(), |n| format!("{n:.1}%")),
    );
    let full = format!(
        "{icon} {}{} {pct_styled} {} {}",
        palette.fg(bar_stress, &g.bar_filled.repeat(filled)),
        palette.fg(Dim, &g.bar_empty.repeat(12 - filled)),
        palette.fg(Dim, "·"),
        palette.fg(Text, &format!("{used}/{}", format_count(capacity))),
    );
    if visible_width(&full) + min_left + usize::from(min_left > 0) > width {
        format!("{icon} {pct_styled}")
    } else {
        full
    }
}

fn parts_width(parts: &[Part]) -> usize {
    parts
        .iter()
        .map(|part| visible_width(&part.text))
        .sum::<usize>()
        + parts.len().saturating_sub(1)
}

fn first_row(
    data: &FooterData,
    options: &FooterOptions,
    palette: &Palette,
    mode: ResolvedIconMode,
    width: usize,
) -> String {
    let g = Glyphs::for_mode(mode);
    let mut parts = Vec::new();
    if options.cwd
        && let Some(cwd) = data.cwd.as_deref()
        && let Some(part) = cwd_part(cwd, data.home_dir.as_deref(), g, palette)
    {
        parts.push(part);
    }
    if options.conversation_title
        && let Some(title) = cleaned(data.conversation_title.as_deref())
    {
        parts.push(Part::new(
            format!(
                "{} {}",
                palette.fg(Dim, g.session),
                palette.fg(Text, &title)
            ),
            2,
        ));
    }
    let git = git_group(data, options, g, palette);
    if !git.is_empty() {
        parts.push(Part::new(git, 3));
    }
    if options.runtime
        && let Some(runtime) = &data.runtime
        && let Some(name) = cleaned(Some(&runtime.name))
    {
        let mut text = palette.fg(Success, runtime_symbol(&name, mode));
        for (value, role) in [(&runtime.version, Muted), (&runtime.framework, Dim)] {
            if let Some(value) = cleaned(value.as_deref()) {
                text.push(' ');
                text.push_str(&palette.fg(role, &value));
            }
        }
        parts.push(Part::new(text, 4));
    }

    let right = if options.context && data.context_window_size > 0 {
        let min_left = parts
            .first()
            .filter(|part| part.priority == 0)
            .map_or(0, |part| {
                visible_width(part.compact.as_deref().unwrap_or(&part.text))
            });
        context(data, g, palette, min_left, width)
    } else {
        String::new()
    };
    let left_budget = width.saturating_sub(visible_width(&right) + usize::from(!right.is_empty()));
    if parts_width(&parts) > left_budget {
        for part in &mut parts {
            if let Some(compact) = part.compact.take() {
                part.text = compact;
            }
        }
    }
    while parts.len() > 1 && parts_width(&parts) > left_budget {
        let (index, _) = parts
            .iter()
            .enumerate()
            .max_by_key(|(_, part)| part.priority)
            .unwrap();
        parts.remove(index);
    }
    let left = parts
        .into_iter()
        .map(|part| part.text)
        .collect::<Vec<_>>()
        .join(" ");
    align(&left, &right, width)
}

fn usage_stats(
    data: &FooterData,
    options: &FooterOptions,
    palette: &Palette,
    mode: ResolvedIconMode,
) -> String {
    let g = Glyphs::for_mode(mode);
    let mut stats = Vec::new();
    if options.tokens {
        for (count, glyph, role) in [
            (data.total_input_tokens, g.input, Accent),
            (data.total_output_tokens, g.output, Success),
        ] {
            if let Some(count) = count {
                stats.push(palette.fg(role, &format!("{glyph} {}", format_count(count))));
            }
        }
        if let Some(rate) = percentage(data.current_turn_cache_hit_rate) {
            let role = if rate < 30.0 {
                Error
            } else if rate < 70.0 {
                Warning
            } else {
                Success
            };
            stats.push(palette.fg(role, &format!("{} {rate:.1}%", g.cache_hit)));
        }
    }
    if options.cost
        && let Some(usd) = data.cost_usd.filter(|usd| usd.is_finite() && *usd >= 0.0)
    {
        let prefix = if mode == ResolvedIconMode::Nerd {
            format!("{} ", g.cost)
        } else {
            String::new()
        };
        stats.push(palette.fg(Warning, &format!("{prefix}${usd:.3}")));
    }
    stats.join(&format!(" {} ", palette.fg(Dim, "|")))
}

fn second_row(
    data: &FooterData,
    options: &FooterOptions,
    palette: &Palette,
    mode: ResolvedIconMode,
    width: usize,
) -> String {
    let g = Glyphs::for_mode(mode);
    let display_name = cleaned(data.model_name.as_deref()).unwrap_or_else(|| "no-model".to_owned());
    let mut label = palette.fg(Text, &display_name);
    if let Some(effort) = cleaned(data.model_effort.as_deref())
        && !effort.eq_ignore_ascii_case("off")
        && !display_name
            .to_lowercase()
            .contains(&format!("({})", effort.to_lowercase()))
    {
        label.push(' ');
        label.push_str(&palette.fg(Dim, &effort));
    }
    let mut model_parts = vec![palette.fg(MdLink, g.model), label.clone()];
    let mut badges = Vec::new();
    let cycle_mode = cleaned(data.cycle_mode.as_deref())
        .map(|mode| mode.to_ascii_lowercase())
        .filter(|mode| matches!(mode.as_str(), "plan" | "auto" | "accept-edits"));
    for (name, glyph, role) in [
        (
            data.tool_confirmation_pending.then_some("confirm"),
            g.stall,
            Warning,
        ),
        (cycle_mode.as_deref(), g.fast, Accent),
        (
            data.sandbox_enabled.then_some("sandbox"),
            g.do_not_pause,
            Warning,
        ),
    ] {
        if let Some(name) = name {
            model_parts.push(palette.fg(role, &format!("{glyph} {name}")));
            badges.push(palette.fg(role, glyph));
        }
    }
    let mut stats = usage_stats(data, options, palette, mode);
    let badges = badges.join(" ");
    let badges_width = visible_width(&badges);
    let reserved =
        width.min(badges_width + usize::from(!badges.is_empty()) + visible_width(&label).min(12));
    if visible_width(&stats) + reserved + 1 > width {
        stats = truncate_to_width(&stats, width.saturating_sub(reserved + 1));
    }
    let mut identity = model_parts.join(&palette.fg(Dim, " · "));
    let stats_width = visible_width(&stats);
    let stats_gap = usize::from(stats_width > 0);
    if visible_width(&identity) + stats_width + stats_gap > width {
        let badges_gap = usize::from(!badges.is_empty());
        identity = truncate_to_width(
            &label,
            width.saturating_sub(stats_width + badges_width + stats_gap + badges_gap),
        );
        if !badges.is_empty() {
            if visible_width(&identity) > 0 {
                identity.push(' ');
            }
            identity.push_str(&badges);
        }
    }
    align(&identity, &stats, width)
}

pub fn build_footer_lines(
    palette: &Palette,
    data: &FooterData,
    options: &FooterOptions,
    mode: ResolvedIconMode,
    width: usize,
) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let width = width.min(MAX_TERMINAL_WIDTH);
    [
        first_row(data, options, palette, mode, width),
        second_row(data, options, palette, mode, width),
    ]
    .iter()
    .map(|line| truncate_to_width(line, width))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ColorScheme;

    #[test]
    fn alignment_right_wins_and_wide_left_cells_leave_padding() {
        assert_eq!(align("left", "right", 3), "rig");
        assert_eq!(align("界界", "x", 5), "界  x");
        assert_eq!(align("left", "", 2), "le");
    }

    #[test]
    fn omits_disabled_segments_and_styles_nerd_usage() {
        let palette = Palette::for_scheme(ColorScheme::Terminal);
        let data = FooterData {
            cwd: Some("/work".into()),
            branch: Some(GitBranch::Named("main".into())),
            git: GitStatus {
                modified: 2,
                ..GitStatus::default()
            },
            context_window_size: 1_000,
            model_name: Some("Model".into()),
            model_effort: Some("high".into()),
            total_input_tokens: Some(900),
            cost_usd: Some(0.0),
            ..FooterData::default()
        };
        let options = FooterOptions {
            cwd: false,
            git_branch: false,
            git_status: false,
            context: false,
            tokens: false,
            ..FooterOptions::default()
        };
        let lines = build_footer_lines(&palette, &data, &options, ResolvedIconMode::Nerd, 80);
        assert_eq!(lines[0], "");
        assert!(lines[1].contains(&palette.fg(Dim, "high")));
        assert!(lines[1].ends_with(&palette.fg(Warning, "\u{F155} $0.000")));
    }
}

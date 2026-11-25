//! Core library for rendering a custom Antigravity CLI (`agy`) status line.

pub mod icons;
pub mod input;
pub mod layout;
pub mod probes;
pub mod text;
pub mod theme;

use std::{env, path::Path, thread, time::Duration};

use icons::{IconMode, ResolvedIconMode};
use input::{CurrentUsage, StatusLineInput, parse_input};
use layout::{FooterData, FooterOptions, MAX_TERMINAL_WIDTH, build_footer_lines};
use probes::{GitBranch, GitStatus, RuntimeInfo, git_from_vcs, probe_git, probe_runtime};
use theme::{Palette, load_agy_color_scheme};

fn resolve_home_dir_with(
    is_windows: bool,
    get_env: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let non_empty = |name| get_env(name).filter(|value| !value.is_empty());
    if is_windows && let Some(home) = non_empty("USERPROFILE") {
        return Some(home);
    }
    non_empty("HOME")
}

fn home_dir() -> Option<String> {
    resolve_home_dir_with(cfg!(windows), |name| env::var(name).ok())
}

/// Renders a status line from a raw JSON payload received on `stdin`.
#[must_use]
pub fn render_statusline(raw: &str) -> String {
    let input = parse_input(raw);
    if input.terminal_width.is_some_and(|width| width <= 0) {
        return String::new();
    }
    let palette = Palette::for_scheme(load_agy_color_scheme());
    let mode = IconMode::Auto.resolve();
    let home = home_dir();
    let dir = input.active_dir().map(Path::new).filter(|dir| dir.is_dir());
    let (git, runtime) = dir
        .map(|dir| {
            thread::scope(|scope| {
                let git =
                    scope.spawn(|| probe_git(dir, input.vcs.as_ref(), Duration::from_millis(200)));
                let runtime = probe_runtime(dir, Duration::from_millis(150));
                (git.join().unwrap(), runtime)
            })
        })
        .unwrap_or_default();
    render_from_input(
        &input,
        &FooterOptions::default(),
        &palette,
        mode,
        home.as_deref(),
        git,
        runtime,
    )
}

/// Renders a parsed payload using only caller-supplied environment and probe results.
#[must_use]
pub fn render_from_input(
    input: &StatusLineInput,
    options: &FooterOptions,
    palette: &Palette,
    mode: ResolvedIconMode,
    home_dir: Option<&str>,
    git: Option<(Option<GitBranch>, GitStatus)>,
    runtime: Option<RuntimeInfo>,
) -> String {
    let context = input.context_window.as_ref();
    let model = input.model.as_ref();
    let (branch, git) = git
        .or_else(|| git_from_vcs(input.vcs.as_ref()))
        .unwrap_or_default();
    let data = FooterData {
        cwd: input.active_dir().map(str::to_owned),
        home_dir: home_dir.map(str::to_owned),
        conversation_title: input.conversation_title.clone(),
        branch,
        git,
        runtime,
        context_window_size: context
            .and_then(|ctx| ctx.context_window_size)
            .unwrap_or_default(),
        used_percentage: context.and_then(|ctx| ctx.used_percentage),
        model_name: model.and_then(|model| {
            [&model.display_name, &model.id]
                .into_iter()
                .filter_map(|name| name.as_deref().map(text::clean))
                .find(|name| !name.is_empty())
        }),
        model_effort: model.and_then(|model| model.effort.clone()),
        cycle_mode: input.cycle_mode.clone(),
        sandbox_enabled: input
            .sandbox
            .as_ref()
            .and_then(|sandbox| sandbox.enabled)
            .unwrap_or_default(),
        tool_confirmation_pending: input.tool_confirmation_pending.unwrap_or_default(),
        total_input_tokens: context.and_then(|ctx| ctx.total_input_tokens),
        total_output_tokens: context.and_then(|ctx| ctx.total_output_tokens),
        current_turn_cache_hit_rate: context
            .and_then(|ctx| ctx.current_usage.as_ref())
            .and_then(cache_hit_rate),
        cost_usd: input.cost.as_ref().and_then(|cost| cost.total_usd),
    };
    let width = usize::try_from(input.terminal_width.unwrap_or(80).max(0))
        .unwrap_or(MAX_TERMINAL_WIDTH)
        .min(MAX_TERMINAL_WIDTH);
    build_footer_lines(palette, &data, options, mode, width).join("\n")
}

fn cache_hit_rate(usage: &CurrentUsage) -> Option<f64> {
    let read = usage.cache_read_input_tokens.unwrap_or(0) as f64;
    let prompt_input = usage.input_tokens.unwrap_or(0) as f64
        + usage.cache_creation_input_tokens.unwrap_or(0) as f64
        + read;
    (prompt_input > 0.0).then(|| 100.0 * read / prompt_input)
}

#[cfg(test)]
mod tests {
    use super::resolve_home_dir_with;

    #[test]
    fn windows_home_prefers_userprofile_and_falls_back_to_home() {
        for (profile, home, expected) in [
            (
                Some(r"C:\Users\alice"),
                Some("/home/alice"),
                Some(r"C:\Users\alice"),
            ),
            (None, Some("/home/alice"), Some("/home/alice")),
            (Some(""), Some("/home/alice"), Some("/home/alice")),
            (Some(""), Some(""), None),
            (None, None, None),
        ] {
            let resolved = resolve_home_dir_with(true, |name| {
                match name {
                    "USERPROFILE" => profile,
                    "HOME" => home,
                    _ => panic!("unexpected variable: {name}"),
                }
                .map(str::to_owned)
            });
            assert_eq!(resolved.as_deref(), expected);
        }
    }

    #[test]
    fn unix_home_reads_only_non_empty_home() {
        for (home, expected) in [
            (Some("/home/alice"), Some("/home/alice")),
            (Some(""), None),
            (None, None),
        ] {
            let resolved = resolve_home_dir_with(false, |name| {
                assert_eq!(name, "HOME");
                home.map(str::to_owned)
            });
            assert_eq!(resolved.as_deref(), expected);
        }
    }
}

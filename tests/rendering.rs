use std::{
    io::Write,
    process::{Command, Stdio},
};

use agy_statusline::{
    icons::ResolvedIconMode::{self, Ascii, Nerd},
    input::parse_input,
    layout::{FooterData, FooterOptions, MAX_TERMINAL_WIDTH, build_footer_lines},
    probes::{GitBranch, GitStatus, RuntimeInfo, parse_git_status},
    render_from_input,
    text::{strip_ansi, visible_width},
    theme::{ColorRole, ColorScheme, Palette},
};

fn render(raw: &str, options: &FooterOptions, mode: ResolvedIconMode) -> String {
    render_from_input(
        &parse_input(raw),
        options,
        &Palette::for_scheme(ColorScheme::Terminal),
        mode,
        Some("/home/user"),
        None,
        None,
    )
}

#[test]
fn renders_the_official_payload_without_duplicate_effort_or_sensitive_fields() {
    let raw = include_str!("fixtures/official-statusline.json");
    let input = parse_input(raw);
    assert_eq!(input.active_dir(), Some("/home/user/my-project"));
    assert_eq!(input.agent_state.as_deref(), Some("idle"));
    assert_eq!(input.vcs.as_ref().unwrap().vcs_type.as_deref(), Some("git"));
    let debug = format!("{input:?}");
    for word in [
        "email",
        "transcript_path",
        "developer@example.com",
        "transcript.jsonl",
    ] {
        assert!(!debug.contains(word));
    }

    let output = render(raw, &FooterOptions::default(), Ascii);
    let lines: Vec<_> = output.lines().map(strip_ansi).collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("@ ~/my-project"));
    assert!(lines[0].ends_with("# ##---------- 14.2% · 149.3K/1M"));
    assert!(lines[1].starts_with("M · Gemini 4 Argon (Medium) · > plan"));
    assert!(lines[1].ends_with("↑ 88.2K | ↓ 61.1K | CH 24.8% | $0.421"));
    assert!(lines.iter().all(|line| visible_width(line) == 111));
}

#[test]
fn handles_empty_malformed_and_startup_input() {
    for raw in ["", "{broken", "null", "{}"] {
        assert_eq!(
            strip_ansi(&render(raw, &FooterOptions::default(), Ascii)),
            "\nM · no-model"
        );
    }
    let raw = r#"{"model":null,"context_window":{"context_window_size":1000,
        "used_percentage":null,"current_usage":null},"agent_state":"future-state",
        "unknown":{"email":"placeholder@example.test"}}"#;
    let input = parse_input(raw);
    assert_eq!(input.agent_state.as_deref(), Some("future-state"));
    assert!(input.model.is_none());
    assert!(input.context_window.unwrap().current_usage.is_none());
    let output = strip_ansi(&render(raw, &FooterOptions::default(), Ascii));
    let (first, second) = output.split_once('\n').unwrap();
    assert!(first.ends_with("# ------------ ? · ?/1K"));
    assert_eq!(visible_width(first), 80);
    assert_eq!(second, "M · no-model");
}

#[test]
fn sanitizes_untrusted_text_and_handles_home_components() {
    let options = FooterOptions {
        conversation_title: true,
        ..FooterOptions::default()
    };
    let raw = r#"{"cwd":"/ignored","workspace":{"current_dir":"/home/user2/\u001b[4m項目\u001b[0m\n開発"},
        "conversation_title":"\u001b]0;discard\u0007設計\t会議",
        "vcs":{"type":"future-vcs","branch":"\u001b[2J分支\rfeature","dirty":true},
        "model":{"id":"モデル\nID","display_name":"\u001b[2J","effort":"OFF"},
        "cycle_mode":"\u001b[1mplan\u001b[0m","terminal_width":100}"#;
    let output = render(raw, &options, Ascii);
    assert!(!output.contains("\x1b[2J"));
    assert!(!output.contains("\x1b]"));
    let plain = strip_ansi(&output);
    assert_eq!(
        plain,
        "@ /home/user2/項目 開発 s 設計 会議 * 分支 feature [!1]\nM · モデル ID · > plan"
    );
    for (path, expected) in [
        ("/home/user", "@ ~"),
        ("/", "@ /"),
        ("/home/user2", "@ /home/user2"),
    ] {
        let raw = serde_json::json!({"cwd":path}).to_string();
        assert_eq!(
            strip_ansi(&render(&raw, &options, Ascii)).lines().next(),
            Some(expected)
        );
    }
}

#[test]
fn strips_control_strings_and_c1_csi_before_shortening_cwd() {
    let raw = r#"{
        "cwd":" \u001bP DCS body \u001b\\\u009b38;2;1;2;3m/home/user/項目\u001b[0m ",
        "conversation_title":"設計\u001b_APC body\u001b\\\u001b^PM body\u001b\\\u001bXSOS body\u001b\\会議",
        "vcs":{"branch":"\u009b1 qmain\u001bPunterminated"}
    }"#;
    let options = FooterOptions {
        conversation_title: true,
        ..FooterOptions::default()
    };
    let output = render(raw, &options, Ascii);
    assert!(!output.contains('\u{009b}'));
    assert_eq!(
        strip_ansi(&output),
        "@ ~/項目 s 設計会議 * main\nM · no-model"
    );
    assert_eq!(visible_width("項\u{009b}1 q目\x1b_ignored\x1b\\"), 4);
}

#[test]
fn caps_huge_widths_and_omits_explicit_non_positive_widths() {
    let options = FooterOptions::default();
    for width in [0, -12] {
        let raw = format!(r#"{{"terminal_width":{width},"cwd":"/work"}}"#);
        let input = parse_input(&raw);
        assert_eq!(input.active_dir(), Some("/work"));
        assert_eq!(input.terminal_width, Some(width));
        assert!(render(&raw, &options, Ascii).is_empty());
    }
    let raw = r#"{"terminal_width":1000000000000,"cwd":"/work",
        "context_window":{"context_window_size":1000},"cost":{"total_usd":1}}"#;
    let output = render(raw, &options, Ascii);
    assert_eq!(
        output.lines().map(visible_width).collect::<Vec<_>>(),
        [MAX_TERMINAL_WIDTH; 2]
    );
}

#[test]
fn keeps_full_context_at_exact_width_without_cwd() {
    let expected = "# ######------ 50.0% · 500K/1M";
    let raw = serde_json::json!({
        "conversation_title": "A title wider than the entire footer",
        "context_window": {"context_window_size": 1000000, "used_percentage": 50},
        "terminal_width": visible_width(expected)
    })
    .to_string();
    let options = FooterOptions {
        conversation_title: true,
        ..FooterOptions::default()
    };
    let output = strip_ansi(&render(&raw, &options, Ascii));
    assert_eq!(output.lines().next(), Some(expected));
}

#[test]
fn narrow_identity_uses_all_available_cells_and_keeps_badges_flush_left() {
    let options = FooterOptions::default();
    for (width, model, expected) in [(1, "Model", "!"), (3, "界", "!"), (8, "Model", "Model !")] {
        let raw = serde_json::json!({
            "terminal_width": width,
            "model": {"display_name": model},
            "tool_confirmation_pending": true
        })
        .to_string();
        let output = strip_ansi(&render(&raw, &options, Ascii));
        assert_eq!(output.lines().nth(1), Some(expected));
    }
    let raw = r#"{"terminal_width":5,"model":{"display_name":"Model"}}"#;
    assert_eq!(strip_ansi(&render(raw, &options, Ascii)), "\nModel");
}

#[test]
fn renders_partial_usage_and_only_known_cycle_modes() {
    let options = FooterOptions::default();
    for (raw, expected_stats) in [
        (
            r#"{"context_window":{"total_input_tokens":1234},"cycle_mode":"unknown"}"#,
            "↑ 1.2K",
        ),
        (r#"{"context_window":{"total_output_tokens":0}}"#, "↓ 0"),
        (
            r#"{"context_window":{"current_usage":{"input_tokens":50,"cache_read_input_tokens":50}}}"#,
            "CH 50.0%",
        ),
    ] {
        let output = strip_ansi(&render(raw, &options, Ascii));
        let second = output.lines().nth(1).unwrap();
        assert!(second.ends_with(expected_stats));
        assert_eq!(
            second.strip_suffix(expected_stats).unwrap().trim_end(),
            "M · no-model"
        );
    }
    for (mode, normalized) in [
        ("PLAN", "plan"),
        (" Auto ", "auto"),
        ("Accept-Edits", "accept-edits"),
    ] {
        let raw = serde_json::json!({"cycle_mode": mode}).to_string();
        assert_eq!(
            strip_ansi(&render(&raw, &options, Ascii)),
            format!("\nM · no-model · > {normalized}")
        );
    }
}

#[test]
fn adapts_rows_at_each_width_in_both_icon_modes() {
    let data = FooterData {
        cwd: Some("/home/user/deep/aaaaaaaaaaaaaaaaaaaaaaaaaaaa/項目".into()),
        home_dir: Some("/home/user".into()),
        conversation_title: Some("設計ノ".into()),
        branch: Some(GitBranch::Named("main".into())),
        git: GitStatus {
            modified: 2,
            staged: 1,
            untracked: 3,
            ahead: 2,
            behind: 1,
            ..GitStatus::default()
        },
        runtime: Some(RuntimeInfo {
            name: "rust".into(),
            version: Some("1.100.0".into()),
            framework: Some("Next.js".into()),
        }),
        context_window_size: 1_000_000,
        used_percentage: Some(50.0),
        model_name: Some("Gemini 4 Argon (Medium)".into()),
        model_effort: Some("medium".into()),
        cycle_mode: Some("auto".into()),
        sandbox_enabled: true,
        tool_confirmation_pending: true,
        total_input_tokens: Some(150_000),
        total_output_tokens: Some(1_500),
        current_turn_cache_hit_rate: Some(70.0),
        cost_usd: Some(1.25),
    };
    let options = FooterOptions {
        conversation_title: true,
        ..FooterOptions::default()
    };
    let palette = Palette::for_scheme(ColorScheme::Terminal);
    assert!(build_footer_lines(&palette, &data, &options, Ascii, 0).is_empty());
    let full_context = "# ######------ 50.0% · 500K/1M";
    let full_cwd = "@ ~/deep/aaaaaaaaaaaaaaaaaaaaaaaaaaaa/項目";
    let compact_with_git = "@ 項目 s 設計ノ * main [!2 A1 ?3 ^v2/1]";
    for (width, left, right) in [
        (
            160,
            format!("{full_cwd} s 設計ノ * main [!2 A1 ?3 ^v2/1] rust 1.100.0 Next.js"),
            full_context,
        ),
        (
            100,
            format!("{compact_with_git} rust 1.100.0 Next.js"),
            full_context,
        ),
        (72, compact_with_git.into(), full_context),
        (50, "@ 項目 s 設計ノ".into(), full_context),
        (30, "@ 項目 s 設計ノ".into(), "# 50.0%"),
        (12, "@ 項".into(), "# 50.0%"),
    ] {
        for mode in [Ascii, Nerd] {
            let rendered = build_footer_lines(&palette, &data, &options, mode, width);
            assert_eq!(rendered.len(), 2);
            assert!(
                rendered.iter().all(|line| visible_width(line) <= width),
                "{width} {mode:?}"
            );
            if mode == Ascii {
                let first = strip_ansi(&rendered[0]);
                let gap = width - visible_width(&left) - visible_width(right);
                assert_eq!(
                    first,
                    format!("{left}{:gap$}{right}", ""),
                    "{width} columns"
                );
                let second = strip_ansi(&rendered[1]);
                if width >= 100 {
                    assert!(second.starts_with(
                        "M · Gemini 4 Argon (Medium) · ! confirm · > auto · play sandbox"
                    ));
                    assert!(second.ends_with("↑ 150K | ↓ 1.5K | CH 70.0% | $1.250"));
                } else {
                    assert!(second.contains("! > play"));
                    assert!(!second.contains("confirm"));
                    if width == 12 {
                        assert_eq!(second, "Gem ! > play");
                    } else {
                        assert!(second.starts_with("Gem"), "{width} columns: {second:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn formats_all_git_counts_and_clips_wide_branch_names() {
    let data = FooterData {
        branch: Some(GitBranch::Named("一二三四五六七八九十多".into())),
        git: GitStatus {
            conflicted: 1,
            deleted: 2,
            modified: 3,
            renamed: 4,
            staged: 5,
            untracked: 6,
            stashed: 7,
            ahead: 8,
            behind: 9,
            oid: Some("abc".into()),
        },
        ..FooterData::default()
    };
    let palette = Palette::for_scheme(ColorScheme::Terminal);
    let options = FooterOptions::default();
    let lines = build_footer_lines(&palette, &data, &options, Ascii, 100);
    assert_eq!(
        strip_ansi(&lines[0]),
        "* 一二三四五六七八九十 [=1 x2 !3 r4 A5 ?6 S7 ^v8/9]"
    );
}

#[test]
fn distinguishes_a_named_detached_branch_from_detached_head() {
    let palette = Palette::for_scheme(ColorScheme::Terminal);
    for (head, expected_branch, role, expected_label) in [
        (
            "detached",
            GitBranch::Named("detached".into()),
            ColorRole::MdLink,
            "* detached",
        ),
        (
            "(detached)",
            GitBranch::Detached,
            ColorRole::Warning,
            "* HEAD",
        ),
    ] {
        let (branch, git) = parse_git_status(&format!("# branch.head {head}"));
        assert_eq!(branch.as_ref(), Some(&expected_branch));
        let data = FooterData {
            branch,
            git,
            ..FooterData::default()
        };
        let lines = build_footer_lines(&palette, &data, &FooterOptions::default(), Ascii, 80);
        assert_eq!(lines[0], palette.fg(role, expected_label));
    }
}

#[test]
fn validates_numeric_readings_and_context_stress() {
    let options = FooterOptions::default();
    let palette = Palette::for_scheme(ColorScheme::Terminal);
    for (used, label, stress, bar_stress) in [
        (f64::NAN, "?", ColorRole::Accent, ColorRole::Success),
        (-2.0, "0.0%", ColorRole::Accent, ColorRole::Success),
        (50.0, "50.0%", ColorRole::Accent, ColorRole::Warning),
        (70.0, "70.0%", ColorRole::Warning, ColorRole::Warning),
        (80.0, "80.0%", ColorRole::Warning, ColorRole::Error),
        (90.0, "90.0%", ColorRole::Error, ColorRole::Error),
        (122.0, "100.0%", ColorRole::Error, ColorRole::Error),
    ] {
        let data = FooterData {
            context_window_size: 2_000,
            used_percentage: Some(used),
            current_turn_cache_hit_rate: Some(f64::INFINITY),
            total_input_tokens: Some(0),
            cost_usd: Some(-1.0),
            ..FooterData::default()
        };
        let lines = build_footer_lines(&palette, &data, &options, Ascii, 80);
        assert!(lines[0].contains(&palette.fg(stress, label)));
        if used >= 50.0 {
            let bar = "#".repeat(((used.min(100.0) * 12.0) / 100.0).round() as usize);
            assert!(lines[0].contains(&palette.fg(bar_stress, &bar)));
        }
        assert!(strip_ansi(&lines[1]).ends_with("↑ 0"));
    }
}

#[test]
fn binary_exits_successfully_and_terminates_output_with_a_newline() {
    let output = run_binary("{malformed");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        strip_ansi(&String::from_utf8(output.stdout).unwrap()),
        "\nM · no-model\n"
    );
}

#[test]
fn binary_omits_sensitive_fields_from_the_official_payload() {
    let output = run_binary(include_str!("fixtures/official-statusline.json"));
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("Gemini 4 Argon (Medium)"));
    for secret in ["developer@example.com", "transcript.jsonl"] {
        assert!(!stdout.contains(secret));
        assert!(!stderr.contains(secret));
    }
}

fn run_binary(raw: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agy-statusline"))
        .env_remove("HOME")
        .env_remove("HERDR_ENV")
        .env_remove("TERM")
        .env_remove("WT_SESSION")
        .env_remove("TERM_PROGRAM")
        .env_remove("LC_TERMINAL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(raw.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

use std::{
    ffi::OsStr,
    io::Write,
    process::{Command, Output, Stdio},
};

fn run(args: &[impl AsRef<OsStr>], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agy-statusline"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    if !input.is_empty() {
        stdin.write_all(input.as_bytes()).unwrap();
    }
    drop(stdin);
    child.wait_with_output().unwrap()
}

#[test]
fn title_renders_the_official_payload_without_sensitive_fields() {
    let output = run(
        &["title"],
        include_str!("fixtures/official-statusline.json"),
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"[my-project] Explain the factorial function\n"
    );
    assert!(output.stderr.is_empty());
    for stream in [&output.stdout, &output.stderr] {
        let text = std::str::from_utf8(stream).unwrap();
        for sensitive in ["developer@example.com", "transcript.jsonl"] {
            assert!(!text.contains(sensitive));
        }
    }
}

#[test]
fn title_is_silent_for_unusable_input() {
    for input in ["{broken", "{}"] {
        let output = run(&["title"], input);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn help_lists_the_title_command() {
    for flag in ["--help", "-h"] {
        let output = run(&[flag], "");
        assert_eq!(output.status.code(), Some(0));
        assert!(
            std::str::from_utf8(&output.stdout)
                .unwrap()
                .contains("Usage: agy-statusline [title]")
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn version_prints_the_package_version() {
    for flag in ["--version", "-V"] {
        let output = run(&[flag], "");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            output.stdout,
            concat!("agy-statusline ", env!("CARGO_PKG_VERSION"), "\n").as_bytes()
        );
        assert!(output.stderr.is_empty());
    }
}

fn assert_usage_error(output: Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("Usage:"));
}

#[test]
fn invalid_arguments_report_a_usage_error() {
    for args in [&["unknown"][..], &["--unknown"], &["title", "extra"]] {
        assert_usage_error(run(args, ""));
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_argument_reports_a_usage_error() {
    use std::os::unix::ffi::OsStrExt;

    assert_usage_error(run(&[OsStr::from_bytes(b"\xff")], ""));
}

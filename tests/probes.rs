use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use agy_statusline::{
    input::VcsInfo,
    probes::{
        GitBranch, detect_runtime_markers, probe_git, probe_runtime, run_command_with_timeout,
    },
    theme::{ColorScheme, load_agy_color_scheme_from_path},
};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("agy-statusline-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("creating test directory: {error}"),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, content: &str) {
        fs::write(self.0.join(name), content).unwrap();
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn reads_color_scheme_from_the_top_level() {
    let dir = TestDir::new();
    let path = dir.path().join("settings.json");
    assert_eq!(
        load_agy_color_scheme_from_path(&path),
        ColorScheme::Terminal
    );
    for (raw, expected) in [
        (
            r#"{"colorScheme":" TOKYO Night ","other":[0]}"#,
            ColorScheme::TokyoNight,
        ),
        (r#"{"other":{"colorScheme":"dark"}}"#, ColorScheme::Terminal),
        (r#"{"colorScheme":false}"#, ColorScheme::Terminal),
        (r#"{"colorScheme":"unknown"}"#, ColorScheme::Terminal),
        (r#"{"colorScheme":"dark""#, ColorScheme::Terminal),
        (r#"["dark"]"#, ColorScheme::Terminal),
    ] {
        dir.write("settings.json", raw);
        assert_eq!(load_agy_color_scheme_from_path(&path), expected);
    }
}

#[test]
fn uses_vcs_fallback_outside_a_repo_and_prefers_real_git_status_inside() {
    let dir = TestDir::new();
    let timeout = Duration::from_secs(2);
    let mut vcs = VcsInfo {
        vcs_type: Some("git".into()),
        branch: Some("fallback".into()),
        dirty: Some(true),
    };
    assert!(probe_git(dir.path(), None, timeout).is_none());
    let (branch, git) = probe_git(dir.path(), Some(&vcs), timeout).unwrap();
    assert_eq!(branch, Some(GitBranch::Named("fallback".into())));
    assert_eq!(git.modified, 1);

    run_command_with_timeout(
        "git",
        &["init", "--initial-branch=detached"],
        dir.path(),
        Duration::from_secs(2),
    )
    .unwrap();
    dir.write("untracked.txt", "content");
    let (branch, git) = probe_git(dir.path(), Some(&vcs), timeout).unwrap();
    assert_eq!(branch, Some(GitBranch::Named("detached".into())));
    assert_eq!(git.oid.as_deref(), Some("(initial)"));
    assert_eq!(git.modified, 0);
    assert_eq!(git.untracked, 1);

    vcs.vcs_type = Some("jj".into());
    let (branch, git) = probe_git(dir.path(), Some(&vcs), timeout).unwrap();
    assert_eq!(branch, Some(GitBranch::Named("fallback".into())));
    assert_eq!(git.modified, 1);
    assert_eq!(git.untracked, 0);
}

#[test]
fn detects_runtime_and_framework_in_catalog_order() {
    let dir = TestDir::new();
    dir.write("vite.config.ts", "");
    assert_eq!(detect_runtime_markers(dir.path()), None);
    dir.write("Cargo.toml", "");
    assert_eq!(
        detect_runtime_markers(dir.path()),
        Some(("rust", "rustc", ["--version"].as_slice(), Some("Vite")))
    );
    let runtime = probe_runtime(dir.path(), Duration::from_secs(1)).unwrap();
    assert_eq!(runtime.name, "rust");
    assert_eq!(runtime.framework.as_deref(), Some("Vite"));
    assert!(runtime.version.is_some());
    let runtime = probe_runtime(dir.path(), Duration::ZERO).unwrap();
    assert!(runtime.version.is_none());

    for marker in [
        "package.json",
        "deno.jsonc",
        "bun.lockb",
        "nuxt.config.js",
        "next.config.mjs",
    ] {
        dir.write(marker, "");
    }
    assert_eq!(
        detect_runtime_markers(dir.path()),
        Some(("bun", "bun", ["--version"].as_slice(), Some("Next.js")))
    );
}

#[cfg(unix)]
#[test]
fn drains_large_stdout_and_bounds_child_and_pipe_timeouts() {
    use std::time::Instant;

    let dir = TestDir::new();
    let output = run_command_with_timeout(
        "sh",
        &["-c", "yes abc | head -n 32768"],
        dir.path(),
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(output.len(), 131_072);
    let capped = run_command_with_timeout(
        "sh",
        &["-c", "yes abc | head -n 131072; exit 0"],
        dir.path(),
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(capped.len(), 256 * 1024);
    assert_eq!(
        run_command_with_timeout(
            "sh",
            &["-c", "printf ignored; exit 2"],
            dir.path(),
            Duration::from_secs(2)
        ),
        None
    );
    for script in ["exec sleep 2", "sleep 2 &"] {
        let start = Instant::now();
        let result =
            run_command_with_timeout("sh", &["-c", script], dir.path(), Duration::from_millis(40));
        assert!(result.is_none(), "{script}");
        assert!(start.elapsed() < Duration::from_secs(1), "{script}");
    }
}

#[cfg(unix)]
#[test]
fn ignores_a_settings_fifo_without_waiting_for_a_writer() {
    use std::{sync::mpsc, thread};

    let dir = TestDir::new();
    run_command_with_timeout(
        "mkfifo",
        &["settings.json"],
        dir.path(),
        Duration::from_secs(2),
    )
    .expect("create settings FIFO");
    let path = dir.path().join("settings.json");
    let (send, receive) = mpsc::channel();
    thread::spawn(move || send.send(load_agy_color_scheme_from_path(&path)).ok());
    assert_eq!(
        receive.recv_timeout(Duration::from_secs(1)),
        Ok(ColorScheme::Terminal)
    );
}

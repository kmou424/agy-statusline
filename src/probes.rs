use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::input::VcsInfo;

#[derive(Debug, PartialEq, Eq)]
pub enum GitBranch {
    Named(String),
    Detached,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GitStatus {
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
    pub renamed: usize,
    pub deleted: usize,
    pub conflicted: usize,
    pub stashed: usize,
    pub ahead: usize,
    pub behind: usize,
    pub oid: Option<String>,
}

pub fn parse_git_status(text: &str) -> (Option<GitBranch>, GitStatus) {
    let mut branch = None;
    let mut status = GitStatus::default();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            branch = Some(if head == "(detached)" {
                GitBranch::Detached
            } else {
                GitBranch::Named(head.to_owned())
            });
        } else if let Some(oid) = line.strip_prefix("# branch.oid ") {
            status.oid = Some(oid.to_owned());
        } else if let Some(divergence) = line.strip_prefix("# branch.ab +")
            && let Some((ahead, behind)) = divergence.split_once(" -")
            && let (Ok(ahead), Ok(behind)) = (ahead.parse(), behind.parse())
        {
            status.ahead = ahead;
            status.behind = behind;
        } else if let Some(stash) = line.strip_prefix("# stash ") {
            status.stashed = stash.parse().unwrap_or(0);
        } else {
            match line.as_bytes() {
                [b'?', b' ', ..] => status.untracked += 1,
                [b'u', b' ', ..] => status.conflicted += 1,
                [b'1' | b'2', b' ', x, y, b' ', ..] => {
                    status.staged += usize::from(*x != b'.');
                    status.modified += usize::from(*y == b'M');
                    status.deleted += usize::from(*x == b'D' || *y == b'D');
                    status.renamed += usize::from(*x == b'R' || *y == b'R');
                }
                _ => {}
            }
        }
    }
    (branch, status)
}

#[derive(Debug, PartialEq, Eq)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: Option<String>,
    pub framework: Option<String>,
}

type Markers = &'static [&'static str];

const RUNTIME_CATALOG: &[(&str, Markers, &str, &[&str])] = &[
    ("bun", &["bun.lock", "bun.lockb"], "bun", &["--version"]),
    ("deno", &["deno.json", "deno.jsonc"], "deno", &["--version"]),
    ("nodejs", &["package.json"], "node", &["--version"]),
    ("rust", &["Cargo.toml"], "rustc", &["--version"]),
    ("go", &["go.mod"], "go", &["version"]),
    (
        "python",
        &["pyproject.toml", "requirements.txt", "Pipfile"],
        "python3",
        &["--version"],
    ),
    ("kotlin", &["build.gradle.kts"], "java", &["--version"]),
    ("java", &["pom.xml", "build.gradle"], "java", &["--version"]),
    ("ruby", &["Gemfile"], "ruby", &["--version"]),
    ("php", &["composer.json"], "php", &["--version"]),
    ("swift", &["Package.swift"], "swift", &["--version"]),
    (
        "dotnet",
        &["global.json", "Directory.Build.props"],
        "dotnet",
        &["--version"],
    ),
    ("elixir", &["mix.exs"], "elixir", &["--version"]),
    (
        "cpp",
        &["CMakeLists.txt", "meson.build"],
        "c++",
        &["--version"],
    ),
];

const FRAMEWORK_CATALOG: &[(&str, Markers)] = &[
    (
        "Next.js",
        &["next.config.js", "next.config.mjs", "next.config.ts"],
    ),
    ("Nuxt", &["nuxt.config.ts", "nuxt.config.js"]),
    ("SvelteKit", &["svelte.config.js", "svelte.config.ts"]),
    ("Angular", &["angular.json"]),
    (
        "Vite",
        &["vite.config.ts", "vite.config.js", "vite.config.mjs"],
    ),
    ("Django", &["manage.py"]),
    ("Laravel", &["artisan"]),
    ("Rails", &["config/application.rb", "bin/rails"]),
];

type RuntimeMatch = (
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
);

pub fn detect_runtime_markers(dir: &Path) -> Option<RuntimeMatch> {
    let matches = |markers: &[&str]| markers.iter().any(|marker| dir.join(marker).exists());
    let &(name, _, program, args) = RUNTIME_CATALOG
        .iter()
        .find(|(_, markers, _, _)| matches(markers))?;
    let framework = FRAMEWORK_CATALOG
        .iter()
        .find(|(_, markers)| matches(markers))
        .map(|&(name, _)| name);
    Some((name, program, args, framework))
}

pub fn extract_version(output: &str) -> Option<String> {
    let bytes = output.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let first_end = i;
        while bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
            i += 2;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
        if i > first_end {
            return Some(output[start..i].to_owned());
        }
    }
    None
}

pub fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Option<String> {
    let start = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let (send, receive) = mpsc::channel();
    let reader = child.stdout.take().and_then(|stdout| {
        thread::Builder::new()
            .spawn(move || {
                let mut bytes = Vec::new();
                if stdout.take(256 * 1024).read_to_end(&mut bytes).is_ok() {
                    let _ = send.send(bytes);
                }
            })
            .ok()
    });
    if reader.is_some() {
        while let Some(remaining) = timeout.checked_sub(start.elapsed()) {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => {
                    // Descendants can inherit stdout even after the child exits.
                    return receive
                        .recv_timeout(timeout.saturating_sub(start.elapsed()))
                        .ok()
                        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                }
                Ok(Some(_)) => return None,
                Ok(None) => thread::sleep(remaining.min(Duration::from_millis(3))),
                Err(_) => break,
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    None
}

pub(crate) fn git_from_vcs(vcs: Option<&VcsInfo>) -> Option<(Option<GitBranch>, GitStatus)> {
    let vcs = vcs?;
    let branch = vcs.branch.as_ref().filter(|branch| !branch.is_empty());
    if branch.is_none() && vcs.dirty.is_none() {
        return None;
    }
    Some((
        branch.cloned().map(GitBranch::Named),
        GitStatus {
            modified: usize::from(vcs.dirty == Some(true)),
            ..GitStatus::default()
        },
    ))
}

pub fn probe_git(
    dir: &Path,
    vcs: Option<&VcsInfo>,
    timeout: Duration,
) -> Option<(Option<GitBranch>, GitStatus)> {
    if vcs
        .and_then(|vcs| vcs.vcs_type.as_deref())
        .is_none_or(|kind| kind == "git")
        && let Some(text) = run_command_with_timeout(
            "git",
            &["status", "--porcelain=v2", "--branch", "--show-stash"],
            dir,
            timeout,
        )
    {
        return Some(parse_git_status(&text));
    }
    git_from_vcs(vcs)
}

pub fn probe_runtime(dir: &Path, timeout: Duration) -> Option<RuntimeInfo> {
    let (name, program, args, framework) = detect_runtime_markers(dir)?;
    let version = run_command_with_timeout(program, args, dir, timeout)
        .and_then(|output| extract_version(&output));
    Some(RuntimeInfo {
        name: name.to_owned(),
        version,
        framework: framework.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_counts_with_overlapping_statuses() {
        let (branch, status) = parse_git_status(
            "# branch.oid 123456\n# branch.head (detached)\n# branch.ab +12 -3\n\
             # stash 4\n? untracked\nu UU conflicted\n1 MM modified\n\
             2 R. renamed\n1 D. deleted\n1 .D removed\n1 .M changed\n1 \n",
        );
        assert_eq!(branch, Some(GitBranch::Detached));
        assert_eq!(
            status,
            GitStatus {
                oid: Some("123456".into()),
                ahead: 12,
                behind: 3,
                stashed: 4,
                untracked: 1,
                conflicted: 1,
                modified: 2,
                renamed: 1,
                deleted: 2,
                staged: 3,
            },
        );
        assert_eq!(
            parse_git_status("# branch.head feat/topic").0,
            Some(GitBranch::Named("feat/topic".into()))
        );
    }

    #[test]
    fn scans_for_first_dotted_numeric_run() {
        for (input, expected) in [
            ("  go version go1.23.4 linux/amd64", Some("1.23.4")),
            (
                "エディション 17; rustc 1.100.0-nightly 9.8",
                Some("1.100.0"),
            ),
            ("0..42.12.", Some("42.12")),
            ("9. x86_64", None),
            ("", None),
        ] {
            assert_eq!(extract_version(input).as_deref(), expected);
        }
    }
}

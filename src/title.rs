use crate::input::{StatusLineInput, parse_input};

/// Renders a plain-text terminal title from a raw JSON payload.
#[must_use]
pub fn render_title(raw: &str) -> String {
    let input = parse_input(raw);
    render_title_from_input(&input)
}

/// Renders a plain-text terminal title from a parsed payload.
#[must_use]
pub fn render_title_from_input(input: &StatusLineInput) -> String {
    let dir_name = input.active_dir().and_then(extract_dir_name);
    let session_title = input
        .conversation_title
        .as_deref()
        .map(clean_single_line)
        .filter(|s| !s.is_empty());

    match (dir_name, session_title) {
        (Some(dir), Some(title)) => format!("[{dir}] {title}"),
        (Some(dir), None) => format!("[{dir}]"),
        (None, Some(title)) => title,
        (None, None) => String::new(),
    }
}

fn clean_single_line(raw: &str) -> String {
    crate::text::clean(raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_dir_name(raw_dir: &str) -> Option<String> {
    let cleaned = clean_single_line(raw_dir);
    if cleaned.is_empty() {
        return None;
    }

    let trimmed = cleaned.trim_end_matches(['/', '\\']);
    let name = trimmed
        .rsplit(['/', '\\'])
        .map(str::trim)
        .find(|seg| !seg.is_empty())
        .unwrap_or("/");
    Some(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_directory_names_independently_of_host_os() {
        for (path, expected) in [
            ("/Users/alice/repo", "[repo]"),
            ("/Users/alice/repo/", "[repo]"),
            ("/", "[/]"),
            ("///", "[/]"),
            ("\\", "[/]"),
            ("/ /", "[/]"),
            (r"C:\Users\alice\repo\", "[repo]"),
            (r"C:\", "[C:]"),
            ("C:/", "[C:]"),
            (r"\\server\share\repo", "[repo]"),
            (r"\\server\share\", "[share]"),
            ("my-repo", "[my-repo]"),
        ] {
            let input = StatusLineInput {
                cwd: Some(path.to_owned()),
                ..StatusLineInput::default()
            };
            assert_eq!(render_title_from_input(&input), expected, "{path:?}");
        }
    }

    #[test]
    fn renders_available_fields_and_uses_the_active_directory() {
        for (raw, expected) in [
            (
                r#"{"cwd":"/other","workspace":{"current_dir":"/repo"},"conversation_title":"Session","terminal_width":0}"#,
                "[repo] Session",
            ),
            (
                r#"{"cwd":"/repo","workspace":{"current_dir":""},"conversation_title":"  \n\t \u001b[0m"}"#,
                "[repo]",
            ),
            (
                r#"{"cwd":" \u0000 ","conversation_title":"Session"}"#,
                "Session",
            ),
            (r#"{"workspace":{"project_dir":"/other"}}"#, ""),
            ("{}", ""),
            ("", ""),
            ("{broken", ""),
        ] {
            assert_eq!(render_title(raw), expected, "{raw:?}");
        }
    }

    #[test]
    fn strips_terminal_sequences_and_collapses_whitespace_in_both_fields() {
        let raw = r#"{
            "cwd":" /parent/\u001b[31m項目\u001b[0m\n\u2003\u001b]0;ignored\u0007開\u0000発/ ",
            "conversation_title":" \u001bPdiscard\u001b\\設\u001b_discard\u001b\\計\r\n\t\u00a0\u001b^discard\u001b\\\u001bXdiscard\u001b\\\u009b2J会\u007f議 "
        }"#;
        assert_eq!(render_title(raw), "[項目 開発] 設計 会議");
    }
}

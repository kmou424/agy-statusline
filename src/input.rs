use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StatusLineInput {
    pub cwd: Option<String>,
    pub conversation_title: Option<String>,
    pub model: Option<ModelInfo>,
    pub workspace: Option<WorkspaceInfo>,
    pub context_window: Option<ContextWindow>,
    pub exceeds_200k_tokens: Option<bool>,
    pub agent_state: Option<String>,
    pub vcs: Option<VcsInfo>,
    pub cycle_mode: Option<String>,
    pub sandbox: Option<SandboxInfo>,
    pub tool_confirmation_pending: Option<bool>,
    pub terminal_width: Option<i64>,
    pub cost: Option<CostInfo>,
}

impl StatusLineInput {
    pub fn active_dir(&self) -> Option<&str> {
        self.workspace
            .as_ref()
            .and_then(|workspace| workspace.current_dir.as_deref())
            .filter(|dir| !dir.is_empty())
            .or_else(|| self.cwd.as_deref().filter(|dir| !dir.is_empty()))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ModelInfo {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub effort: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct WorkspaceInfo {
    pub current_dir: Option<String>,
    pub project_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ContextWindow {
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct VcsInfo {
    #[serde(rename = "type")]
    pub vcs_type: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SandboxInfo {
    pub enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct CostInfo {
    pub total_usd: Option<f64>,
    pub subagent_usd: Option<f64>,
    pub estimated: Option<bool>,
}

pub fn parse_input(raw: &str) -> StatusLineInput {
    let raw = raw.trim();
    if raw.starts_with('{') {
        serde_json::from_str(raw).unwrap_or_default()
    } else {
        StatusLineInput::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_nonempty_active_directory() {
        for (raw, expected) in [
            (
                r#"{"cwd":"/home","workspace":{"current_dir":"/app"}}"#,
                Some("/app"),
            ),
            (
                r#"{"cwd":"/home","workspace":{"current_dir":""}}"#,
                Some("/home"),
            ),
            (r#"{"cwd":"","workspace":{"project_dir":"/app"}}"#, None),
        ] {
            assert_eq!(parse_input(raw).active_dir(), expected);
        }
    }

    #[test]
    fn defaults_on_unusable_payloads() {
        for raw in ["", "  ", "null", "{}", "{", r#"{"cwd":17}"#, r#"["/dir"]"#] {
            let input = parse_input(raw);
            assert!(input.active_dir().is_none());
            assert!(input.model.is_none());
        }
    }
}

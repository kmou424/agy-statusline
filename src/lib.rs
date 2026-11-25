//! Core library for rendering a custom Antigravity CLI (`agy`) status line.

/// Renders a status line from a raw JSON payload received on `stdin`.
#[must_use]
pub fn render_statusline(_input: &str) -> String {
    format!("agy-statusline {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::render_statusline;

    #[test]
    fn renders_placeholder_output() {
        let output = render_statusline("{}");
        assert!(output.starts_with("agy-statusline "));
    }

    #[test]
    fn handles_empty_input() {
        let output = render_statusline("");
        assert!(!output.is_empty());
    }
}

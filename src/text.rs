use unicode_width::UnicodeWidthChar;

const RESET: &str = "\x1b[0m";

fn csi_len(bytes: &[u8]) -> usize {
    let mut end = 2;
    while matches!(bytes.get(end), Some(0x20..=0x3f)) {
        end += 1;
    }
    end + usize::from(matches!(bytes.get(end), Some(0x40..=0x7e)))
}

fn escape_len(input: &str) -> usize {
    let bytes = input.as_bytes();
    match bytes.get(1) {
        Some(b'[') => csi_len(bytes),
        Some(b']' | b'P' | b'_' | b'^' | b'X') => {
            let mut end = 2;
            while end < bytes.len() {
                match bytes[end] {
                    b'\x07' if bytes[1] == b']' => return end + 1,
                    b'\x1b' if bytes.get(end + 1) == Some(&b'\\') => return end + 2,
                    _ => end += 1,
                }
            }
            end
        }
        _ => {
            let mut end = 1;
            while matches!(bytes.get(end), Some(0x20..=0x2f)) {
                end += 1;
            }
            end + usize::from(matches!(bytes.get(end), Some(0x30..=0x7e)))
        }
    }
}

enum Piece<'a> {
    Char(char),
    Escape(&'a str),
}

fn pieces(mut input: &str) -> impl Iterator<Item = Piece<'_>> {
    std::iter::from_fn(move || {
        let ch = input.chars().next()?;
        if matches!(ch, '\x1b' | '\u{009B}') {
            let len = if ch == '\x1b' {
                escape_len(input)
            } else {
                csi_len(input.as_bytes())
            };
            let (escape, rest) = input.split_at(len);
            input = rest;
            Some(Piece::Escape(escape))
        } else {
            input = &input[ch.len_utf8()..];
            Some(Piece::Char(ch))
        }
    })
}

pub fn strip_ansi(input: &str) -> String {
    pieces(input)
        .filter_map(|piece| match piece {
            Piece::Char(ch) => Some(ch),
            Piece::Escape(_) => None,
        })
        .collect()
}

pub fn clean(input: &str) -> String {
    strip_ansi(input)
        .chars()
        .filter_map(|ch| match ch {
            '\n' | '\r' | '\t' => Some(' '),
            _ if ch.is_control() => None,
            _ => Some(ch),
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

pub fn char_display_width(ch: char) -> usize {
    match ch {
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}' => 1,
        _ => ch.width().unwrap_or(0),
    }
}

pub fn visible_width(input: &str) -> usize {
    pieces(input)
        .map(|piece| match piece {
            Piece::Char(ch) => char_display_width(ch),
            Piece::Escape(_) => 0,
        })
        .sum()
}

pub fn truncate_to_width(input: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if visible_width(input) <= max_width {
        return input.to_owned();
    }
    let mut output = String::new();
    let mut remaining = max_width;
    let mut emitted_escape = false;
    for piece in pieces(input) {
        match piece {
            Piece::Char(ch) => {
                let width = char_display_width(ch);
                if width > remaining {
                    break;
                }
                remaining -= width;
                output.push(ch);
            }
            Piece::Escape(escape) => {
                emitted_escape = true;
                output.push_str(escape);
            }
        }
    }
    if emitted_escape && !output.ends_with(RESET) {
        output.push_str(RESET);
    }
    output
}

pub fn format_count(value: u64) -> String {
    if value < 1_000 {
        return value.to_string();
    }
    for (unit, suffix) in [(1_000, 'K'), (1_000_000, 'M'), (1_000_000_000, 'B')] {
        let step = unit / 10;
        let tenths = value / step + u64::from(value % step >= step / 2);
        if tenths < 10_000 || suffix == 'B' {
            return if tenths.is_multiple_of(10) {
                format!("{}{suffix}", tenths / 10)
            } else {
                format!("{}.{}{suffix}", tenths / 10, tenths % 10)
            };
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_controls_and_unicode_stays_intact() {
        let raw = " \x1b[38;2;1;2;3m項目\x1b[0m\x1b]0;ignored\x07\n\
                   \x1b]8;;url\x1b\\名\x1b]8;;\x1b\\\t\x1bM\x1b(B\x00\u{85} ";
        assert_eq!(strip_ansi(raw), " 項目\n名\t\x00\u{85} ");
        assert_eq!(clean(raw), "項目 名");
        assert_eq!(strip_ansi("a\x1b]unfinished"), "a");
        assert_eq!(strip_ansi("a\x1b[12界"), "a界");
    }

    #[test]
    fn truncates_on_cell_boundaries_and_preserves_styling() {
        let raw = "\x1b[31m界\u{F0000}e\u{301}x\x1b[0m";
        assert_eq!(visible_width(raw), 5);
        assert_eq!(
            truncate_to_width(raw, 4),
            "\x1b[31m界\u{F0000}e\u{301}\x1b[0m"
        );
        assert_eq!(truncate_to_width(raw, 5), raw);
        assert!(truncate_to_width(raw, 0).is_empty());
        assert_eq!(
            truncate_to_width("\x1b[31ma\x1b[0m界", 2),
            "\x1b[31ma\x1b[0m"
        );
        assert_eq!(char_display_width('\u{10FFFD}'), 1);
    }

    #[test]
    fn rounds_and_carries_compact_counts() {
        for (value, expected) in [
            (0, "0"),
            (999, "999"),
            (1_000, "1K"),
            (1_450, "1.5K"),
            (150_000, "150K"),
            (88_244, "88.2K"),
            (999_949, "999.9K"),
            (999_950, "1M"),
            (1_150_000, "1.2M"),
            (999_950_000, "1B"),
            (1_050_000_000, "1.1B"),
            (u64::MAX, "18446744073.7B"),
        ] {
            assert_eq!(format_count(value), expected, "{value}");
        }
    }
}

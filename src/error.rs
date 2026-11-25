use std::fmt;

/// A single recoverable failure. The message is safe to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
}

/// Text safe for one terminal line. Control characters are escaped, not copied raw.
#[must_use]
pub(crate) fn show_text(text: &str) -> String {
    if !text.chars().any(is_unsafe) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if is_unsafe(ch) {
            out.extend(ch.escape_default());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Like [`show_text`] for a message that is already several lines (clap usage
/// errors echo the user's arguments). Only `\n` stays a real line break.
#[must_use]
pub fn show_lines(text: &str) -> String {
    text.split('\n')
        .map(show_text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Control, line/paragraph separator, and invisible format characters (bidi
/// overrides, zero-width space) are never copied raw into text output.
pub(crate) fn is_unsafe(ch: char) -> bool {
    ch.is_control()
        || matches!(
            ch,
            '\u{00AD}'
                | '\u{034F}'
                | '\u{061C}'
                | '\u{115F}'
                | '\u{1160}'
                | '\u{17B4}'
                | '\u{17B5}'
                | '\u{180B}'..='\u{180D}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2065}'
                | '\u{2066}'..='\u{206F}'
                | '\u{2800}'
                | '\u{3164}'
                | '\u{FE00}'..='\u{FE0E}'
                | '\u{FE0F}'
                | '\u{FFA0}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{FEFF}'
                | '\u{E0000}'..='\u{E007F}'
        )
}

impl Error {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: show_text(&message.into()),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::show_text;

    #[test]
    fn invisible_and_bidi_characters_are_escaped() {
        let shown = show_text("a\u{202E}b\u{2028}c\u{200B}d");
        assert_eq!(shown, "a\\u{202e}b\\u{2028}c\\u{200b}d");
        assert!(!shown.contains('\u{202E}'));
        assert_eq!(show_text("plain é"), "plain é");
    }

    #[test]
    fn tag_characters_are_escaped() {
        let shown = show_text("a\u{E0001}b");
        assert!(!shown.contains('\u{E0001}'));
        assert_eq!(shown, "a\\u{e0001}b");
        assert_eq!(show_text("plain é"), "plain é");
    }

    #[test]
    fn blank_and_selector_characters_are_escaped() {
        let shown = show_text("a\u{2800}b\u{3164}c\u{034F}d\u{FE0F}e");
        assert_eq!(shown, "a\\u{2800}b\\u{3164}c\\u{34f}d\\u{fe0f}e");
    }

    #[test]
    fn added_invisible_characters_are_each_escaped() {
        let list: Vec<char> = [0x115F, 0x1160, 0x17B4, 0x17B5, 0xFFA0]
            .into_iter()
            .chain(0x180B..=0x180D)
            .chain(0xFE00..=0xFE0E)
            .chain(0xFFF9..=0xFFFB)
            .map(|c| char::from_u32(c).unwrap())
            .collect();
        assert_eq!(list.len(), 5 + 3 + 15 + 3);
        for ch in list {
            let shown = show_text(&format!("a{ch}b"));
            assert_eq!(shown, format!("a\\u{{{:x}}}b", u32::from(ch)));
            assert!(!shown.contains(ch));
        }
    }
}

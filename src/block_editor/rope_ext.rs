use ropey::{Rope, RopeSlice};

pub trait RopeExt {
    /// the number of characters in line of source.
    /// excludes the linebreak.
    ///
    /// runs in O(log N) time
    fn len_char_for_line(&self, row: usize) -> usize;

    /// finds the characters surrounding an offset (prev, next).
    /// '\0' if no surrounding character.
    ///
    /// runs in O(log N) time
    fn surrounding_chars(&self, cursor_idx: usize) -> (char, char);

    /// the linebreak present or the platform default if none
    ///
    /// runs in O(log N) time
    fn detect_linebreak(&self) -> &'static str;
}

impl RopeExt for Rope {
    fn len_char_for_line(&self, row: usize) -> usize {
        let line = self.line(row);
        if line.len_chars() == 0 {
            return 0;
        };

        // adjust for linebreak
        let linebreak_len = linebreak_of_line(line).map_or(0, |l| l.len());

        line.len_chars() - linebreak_len
    }

    fn surrounding_chars(&self, cursor_idx: usize) -> (char, char) {
        let Some(mut chars) = self.get_chars_at(cursor_idx) else { return ('\0', '\0') };
        let prev = chars.prev();
        if prev.is_some() {
            // if did move back, move back forward
            chars.next();
        }
        let next = chars.next();
        (prev.unwrap_or('\0'), next.unwrap_or('\0'))
    }

    fn detect_linebreak(&self) -> &'static str {
        // check what first linebreak is
        if let Some(line) = self.get_line(0) {
            if let Some(linebreak) = linebreak_of_line(line) {
                return linebreak;
            }
        }

        // if no line breaks, default to platform default
        if cfg!(target_os = "windows") {
            "\r\n"
        } else {
            "\n"
        }
    }
}

pub trait RopeSliceExt {
    /// check last character in rope slice
    ///
    /// runs in O(log N) time
    fn ends_with(&self, c: char) -> bool;

    /// the number of whitespace characters at the start of the slice
    ///
    /// runs in O(M log N) time (N number chars, M length of first line)
    fn whitespace_at_start(&self) -> usize;
}

impl RopeSliceExt for RopeSlice<'_> {
    fn ends_with(&self, c: char) -> bool {
        if self.len_chars() == 0 {
            return false;
        };
        let Some(last) = self.get_char(self.len_chars() - 1) else { return false };
        last == c
    }

    fn whitespace_at_start(&self) -> usize {
        self.chars()
            .take_while(|ch| ch.is_whitespace() && *ch != '\n')
            .count()
    }
}

/// returns linebreak at the end of the slice (if any)
///
/// runs in O(log N) time
fn linebreak_of_line(line: RopeSlice) -> Option<&'static str> {
    let mut c = line.chars_at(line.len_chars());
    if c.prev() == Some('\n') {
        if c.prev() == Some('\r') {
            return Some("\r\n");
        } else {
            return Some("\n");
        }
    }
    None
}

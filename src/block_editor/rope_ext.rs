use ropey::{Rope, RopeSlice};

pub trait RopeExt {
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
            if let Some(linebreak) = linebreak_of_line(&line) {
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

    /// length of the slice, excluding linebreak characters at the end
    ///
    /// runs in O(log N) time
    fn len_chars_no_linebreak(&self) -> usize;

    /// a slice of the rope excluding the linebreak at the end
    ///
    /// runs in O(log N) time
    fn excluding_linebreak(&self) -> Self;
}

impl<'a> RopeSliceExt for RopeSlice<'a> {
    fn ends_with(&self, c: char) -> bool {
        if self.len_chars() == 0 {
            return false;
        };
        let Some(last) = self.get_char(self.len_chars() - 1) else { return false };
        last == c
    }

    fn whitespace_at_start(&self) -> usize {
        self.chars()
            .take_while(|ch| ch.is_whitespace() && *ch != '\n' && *ch != '\r')
            .count()
    }

    fn len_chars_no_linebreak(&self) -> usize {
        if self.len_chars() == 0 {
            return 0;
        };

        // adjust for linebreak
        let linebreak_len = linebreak_of_line(self).map_or(0, |l| l.len());

        self.len_chars() - linebreak_len
    }

    fn excluding_linebreak(&self) -> RopeSlice<'a> {
        let linebreak_len = linebreak_of_line(self).map_or(0, |l| l.len());
        let new_end = self.len_chars() - linebreak_len;
        self.slice(..new_end)
    }
}

/// returns linebreak at the end of the slice (if any)
///
/// runs in O(log N) time
fn linebreak_of_line(line: &RopeSlice) -> Option<&'static str> {
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

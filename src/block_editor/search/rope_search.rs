use ropey::{iter::Chars, Rope};

/// An iterator over simple textual matches in a Rope.
///
/// This implementation is somewhat naive, and could be sped up by using a
/// more sophisticated text searching algorithm such as Boyer-Moore or
/// Knuth-Morris-Pratt.
///
/// Originally pulled from: https://github.com/cessen/ropey/blob/master/examples/search_and_replace.rs
pub struct SearchIter<'a> {
    char_iter: Chars<'a>,
    search_pattern: &'a str,
    search_pattern_char_len: usize,
    cur_index: usize, // The current char index of the search head.
    possible_matches: Vec<std::str::Chars<'a>>, // Tracks where we are in the search pattern for the current possible matches.
}

impl SearchIter<'_> {
    pub fn from_rope_slice<'b>(slice: &'b Rope, search_pattern: &'b str) -> SearchIter<'b> {
        assert!(
            !search_pattern.is_empty(),
            "Can't search using an empty search pattern."
        );
        SearchIter {
            char_iter: slice.chars(),
            search_pattern,
            search_pattern_char_len: search_pattern.chars().count(),
            cur_index: 0,
            possible_matches: Vec::new(),
        }
    }
}

impl Iterator for SearchIter<'_> {
    type Item = (usize, usize);

    // Return the start/end char indices of the next match.
    fn next(&mut self) -> Option<(usize, usize)> {
        #[allow(clippy::while_let_on_iterator)]
        while let Some(next_char) = self.char_iter.next() {
            self.cur_index += 1;

            // Push new potential match, for a possible match starting at the
            // current char.
            self.possible_matches.push(self.search_pattern.chars());

            // Check the rope's char against the next character in each of
            // the potential matches, removing the potential matches that
            // don't match.  We're using indexing instead of iteration here
            // so that we can remove the possible matches as we go.
            let mut i = 0;
            while i < self.possible_matches.len() {
                let pattern_char = self.possible_matches[i].next().unwrap();
                if next_char == pattern_char {
                    if self.possible_matches[i].clone().next().is_none() {
                        // We have a match!  Reset possible matches and
                        // return the successful match's char indices.
                        let char_match_range = (
                            self.cur_index - self.search_pattern_char_len,
                            self.cur_index,
                        );
                        self.possible_matches.clear();
                        return Some(char_match_range);
                    } else {
                        // Match isn't complete yet, move on to the next.
                        i += 1;
                    }
                } else {
                    // Doesn't match, remove it.
                    let _ = self.possible_matches.swap_remove(i);
                }
            }
        }

        None
    }
}

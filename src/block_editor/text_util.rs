/// the number of characters in line of source
pub fn line_len(row: usize, source: &str) -> usize {
    source.lines().nth(row).unwrap_or("").chars().count()
}

pub fn line_count(source: &str) -> usize {
    // add one if the last line is a newline (because the lines method does not include that)
    source.lines().count() + if source.ends_with('\n') { 1 } else { 0 }
}

pub fn detect_linebreak(text: &str) -> &'static str {
    // currently not cached because vscode can it change at any time
    // if a bottleneck, could possibly be cached and updated when vscode sends a change

    if text.contains("\r\n") {
        "\r\n"
    } else if text.contains('\n') {
        "\n"
    } else {
        // if no line breaks, default to platform default
        if cfg!(target_os = "windows") {
            "\r\n"
        } else {
            "\n"
        }
    }
}

/// finds the characters surrounding an offset (prev, next).
/// '\0' if no surrounding character.
pub fn surrounding_chars(offset: usize, source: &str) -> (char, char) {
    let mut chars = source.chars();
    let prev_char = if offset > 0 {
        chars.nth(offset - 1).unwrap_or('\0')
    } else {
        '\0'
    };
    let next_char = chars.next().unwrap_or('\0');
    (prev_char, next_char)
}

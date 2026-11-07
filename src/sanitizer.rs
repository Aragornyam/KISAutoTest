use crate::models::Lang;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanState {
    Normal,
    LineComment,
    BlockComment,
    SingleQuote,
    DoubleQuote,
    BacktickQuote,
    TripleSingleQuote,
    TripleDoubleQuote,
}

pub fn sanitize_lines(lang: Lang, content: &str) -> Vec<String> {
    let chars: Vec<char> = content.chars().collect();
    let mut out = String::with_capacity(content.len());
    let mut state = ScanState::Normal;
    let mut i = 0;
    let is_py = lang == Lang::Python;

    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            out.push('\n');
            if state == ScanState::LineComment
                || state == ScanState::SingleQuote
                || state == ScanState::DoubleQuote
            {
                state = ScanState::Normal;
            }
            i += 1;
            continue;
        }

        match state {
            ScanState::Normal => {
                if is_py && c == '#' {
                    state = ScanState::LineComment;
                    out.push(' ');
                    i += 1;
                } else if !is_py && c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    state = ScanState::LineComment;
                    out.push_str("  ");
                    i += 2;
                } else if !is_py && c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    state = ScanState::BlockComment;
                    out.push_str("  ");
                    i += 2;
                } else if c == '"'
                    && i + 2 < chars.len()
                    && chars[i + 1] == '"'
                    && chars[i + 2] == '"'
                {
                    state = ScanState::TripleDoubleQuote;
                    out.push_str("   ");
                    i += 3;
                } else if is_py
                    && c == '\''
                    && i + 2 < chars.len()
                    && chars[i + 1] == '\''
                    && chars[i + 2] == '\''
                {
                    state = ScanState::TripleSingleQuote;
                    out.push_str("   ");
                    i += 3;
                } else if c == '"' {
                    state = ScanState::DoubleQuote;
                    out.push(' ');
                    i += 1;
                } else if c == '\'' {
                    state = ScanState::SingleQuote;
                    out.push(' ');
                    i += 1;
                } else if (lang == Lang::JavaScript || lang == Lang::Go) && c == '`' {
                    state = ScanState::BacktickQuote;
                    out.push(' ');
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            ScanState::LineComment => {
                out.push(' ');
                i += 1;
            }
            ScanState::BlockComment => {
                if c == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    state = ScanState::Normal;
                    out.push_str("  ");
                    i += 2;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            ScanState::SingleQuote => {
                if c == '\\' && i + 1 < chars.len() && chars[i + 1] != '\n' {
                    out.push_str("  ");
                    i += 2;
                } else if c == '\'' {
                    state = ScanState::Normal;
                    out.push(' ');
                    i += 1;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            ScanState::DoubleQuote => {
                if c == '\\' && i + 1 < chars.len() && chars[i + 1] != '\n' {
                    out.push_str("  ");
                    i += 2;
                } else if c == '"' {
                    state = ScanState::Normal;
                    out.push(' ');
                    i += 1;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            ScanState::BacktickQuote => {
                if c == '\\' && i + 1 < chars.len() && chars[i + 1] != '\n' {
                    out.push_str("  ");
                    i += 2;
                } else if c == '`' {
                    state = ScanState::Normal;
                    out.push(' ');
                    i += 1;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            ScanState::TripleDoubleQuote => {
                if c == '\\' && i + 1 < chars.len() && chars[i + 1] != '\n' {
                    out.push_str("  ");
                    i += 2;
                } else if c == '"'
                    && i + 2 < chars.len()
                    && chars[i + 1] == '"'
                    && chars[i + 2] == '"'
                {
                    state = ScanState::Normal;
                    out.push_str("   ");
                    i += 3;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
            ScanState::TripleSingleQuote => {
                if c == '\\' && i + 1 < chars.len() && chars[i + 1] != '\n' {
                    out.push_str("  ");
                    i += 2;
                } else if c == '\''
                    && i + 2 < chars.len()
                    && chars[i + 1] == '\''
                    && chars[i + 2] == '\''
                {
                    state = ScanState::Normal;
                    out.push_str("   ");
                    i += 3;
                } else {
                    out.push(' ');
                    i += 1;
                }
            }
        }
    }

    out.lines().map(|s| s.to_string()).collect()
}

pub fn is_snake_case(s: &str) -> bool {
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !trimmed.contains("__")
}

pub fn is_upper_snake_case(s: &str) -> bool {
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_camel_case(s: &str) -> bool {
    let trimmed = s.trim_start_matches('$').trim_start_matches('_');
    let Some(first) = trimmed.chars().next() else {
        return true;
    };
    first.is_ascii_lowercase()
        && !trimmed.contains('_')
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '$')
}

pub fn is_pascal_case(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return true;
    };
    first.is_ascii_uppercase() && !s.contains('_') && s.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn is_go_valid_name(s: &str, is_test_file: bool) -> bool {
    if is_test_file
        && (s.starts_with("Test_") || s.starts_with("Benchmark_") || s.starts_with("Example_"))
    {
        return true;
    }
    !s.contains('_')
}

pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut chars = s.trim().chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", collected)
    } else {
        collected
    }
}
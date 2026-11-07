use crate::models::{Lang, ParsedFunction};
use regex::Regex;

pub fn extract_functions(
    lang: Lang,
    raw_lines: &[&str],
    clean_lines: &[String],
) -> Vec<ParsedFunction> {
    if lang == Lang::Python {
        return extract_python_functions(raw_lines, clean_lines);
    }

    let mut funcs = Vec::new();
    let re = match lang {
        Lang::JavaScript => Regex::new(
            r"^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*\(([^)]*)\)|(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?\(([^)]*)\)\s*=>|(?:async\s+|static\s+)*([A-Za-z_$][A-Za-z0-9_$]*)\s*\(([^)]*)\)\s*\{)",
        ).unwrap(),
        Lang::CSharp => Regex::new(
            r"^\s*(?:(?:public|private|protected|internal|static|virtual|override|abstract|async|sealed|new|partial)\s+)+[A-Za-z0-9_<>,\[\]\?\.\s]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]+>)?\s*\(([^)]*)\)",
        ).unwrap(),
        Lang::Java => Regex::new(
            r"^\s*(?:(?:public|private|protected|static|final|abstract|synchronized|default)\s+)+(?:<[^>]+>\s+)?[A-Za-z0-9_<>,\[\]\.\s]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
        ).unwrap(),
        Lang::Go => Regex::new(
            r"^\s*func\s+(?:\([^)]+\)\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^\]]+\])?\s*\(([^)]*)\)",
        ).unwrap(),
        Lang::Kotlin => Regex::new(
            r"^\s*(?:(?:public|private|protected|internal|override|open|suspend|inline|operator|infix)\s+)*fun\s+(?:<[^>]+>\s*)?(?:[A-Za-z0-9_<>?.,\s]+\.)?([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
        ).unwrap(),
        Lang::Rust => Regex::new(
            r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern(?:\s+)*)*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(([^)]*)\)",
        ).unwrap(),
        _ => return funcs,
    };

    let mut i = 0;
    while i < clean_lines.len() {
        if clean_lines[i].trim().is_empty() {
            i += 1;
            continue;
        }

        let mut combined = String::new();
        for offset in 0..6 {
            if i + offset < clean_lines.len() {
                if offset > 0 {
                    combined.push(' ');
                }
                combined.push_str(clean_lines[i + offset].trim());
                if clean_lines[i + offset].contains('{') || clean_lines[i + offset].contains(';') {
                    break;
                }
            }
        }

        if let Some(caps) = re.captures(&combined) {
            let (name, args_str) = extract_name_and_args(&caps);
            if !is_control_keyword(&name) {
                let start_line = i + 1;
                if let Some(end_line) = find_brace_function_end(clean_lines, i) {
                    let (arg_count, params_raw) = parse_arguments(lang, &args_str, start_line);
                    funcs.push(ParsedFunction {
                        name,
                        start_line,
                        end_line,
                        arg_count,
                        params_raw,
                    });
                    i = end_line;
                    continue;
                }
            }
        }
        i += 1;
    }

    funcs
}

fn extract_name_and_args(caps: &regex::Captures) -> (String, String) {
    for pair_idx in [1, 3, 5] {
        if let Some(name_match) = caps.get(pair_idx) {
            let args_match = caps.get(pair_idx + 1).map_or("", |m| m.as_str());
            return (name_match.as_str().to_string(), args_match.to_string());
        }
    }
    (String::new(), String::new())
}

fn is_control_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "for" | "while" | "switch" | "catch" | "synchronized" | "using" | "lock" | "when"
    )
}

fn find_brace_function_end(clean_lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut found_open = false;

    for (idx, clean) in clean_lines.iter().enumerate().skip(start_idx) {
        if !found_open && clean.contains(';') && !clean.contains('{') {
            return None;
        }
        for ch in clean.chars() {
            if ch == '{' {
                depth += 1;
                found_open = true;
            } else if ch == '}' {
                depth -= 1;
                if found_open && depth == 0 {
                    return Some(idx + 1);
                }
            }
        }
    }
    None
}

fn extract_python_functions(raw_lines: &[&str], clean_lines: &[String]) -> Vec<ParsedFunction> {
    let mut funcs = Vec::new();
    let def_re =
        Regex::new(r"^(\s*)(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)").unwrap();

    let mut i = 0;
    while i < clean_lines.len() {
        if let Some(caps) = def_re.captures(&clean_lines[i]) {
            let indent = caps[1].chars().count();
            let name = caps[2].to_string();
            let start_line = i + 1;

            let mut sig_acc = caps[3].to_string();
            let mut sig_end_idx = i;
            while !sig_acc.contains(')') && sig_end_idx + 1 < clean_lines.len() {
                sig_end_idx += 1;
                sig_acc.push(' ');
                sig_acc.push_str(clean_lines[sig_end_idx].trim());
            }
            let args_str = sig_acc.split(')').next().unwrap_or("").to_string();
            let (arg_count, params_raw) = parse_arguments(Lang::Python, &args_str, start_line);

            let mut end_line = sig_end_idx + 1;
            for (j, &next_raw) in raw_lines.iter().enumerate().skip(sig_end_idx + 1) {
                let trimmed = next_raw.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let next_indent = next_raw.chars().take_while(|c| c.is_whitespace()).count();
                if next_indent <= indent {
                    break;
                }
                end_line = j + 1;
            }

            funcs.push(ParsedFunction {
                name,
                start_line,
                end_line,
                arg_count,
                params_raw,
            });
        }
        i += 1;
    }
    funcs
}

pub fn parse_arguments(lang: Lang, raw: &str, line_no: usize) -> (usize, Vec<(String, usize)>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (0, Vec::new());
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    let mut angle_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;

    for ch in trimmed.chars() {
        match ch {
            '<' => {
                angle_depth += 1;
                current.push(ch);
            }
            '>' => {
                angle_depth = (angle_depth - 1).max(0);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = (paren_depth - 1).max(0);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = (bracket_depth - 1).max(0);
                current.push(ch);
            }
            ',' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    let mut extracted = Vec::new();
    for p in parts {
        let p_clean = p.trim();
        if p_clean.is_empty() {
            continue;
        }
        if (lang == Lang::Python
            && (p_clean == "self" || p_clean == "cls" || p_clean == "*" || p_clean == "/"))
            || (lang == Lang::Rust
                && (p_clean == "self" || p_clean == "&self" || p_clean == "&mut self"))
        {
            continue;
        }

        let name = extract_param_name(lang, p_clean);
        if !name.is_empty() {
            extracted.push((name, line_no));
        }
    }

    (extracted.len(), extracted)
}

fn extract_param_name(lang: Lang, param: &str) -> String {
    let without_default = param.split('=').next().unwrap_or(param).trim();
    match lang {
        Lang::Python | Lang::Rust | Lang::Kotlin => {
            let before_colon = without_default
                .split(':')
                .next()
                .unwrap_or(without_default)
                .trim();
            before_colon
                .trim_start_matches("mut ")
                .trim_start_matches("val ")
                .trim_start_matches("var ")
                .trim_start_matches('*')
                .trim()
                .to_string()
        }
        Lang::JavaScript => without_default
            .trim_start_matches("...")
            .split(':')
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
        Lang::CSharp | Lang::Java => without_default
            .split_whitespace()
            .last()
            .unwrap_or("")
            .trim_start_matches('@')
            .to_string(),
        Lang::Go => without_default
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string(),
        Lang::OtherCode => String::new(),
    }
}
use crate::models::{Lang, ParsedFunction, RepoInput, Violation};
use crate::rules::make_violation;
use crate::sanitizer::{
    is_camel_case, is_go_valid_name, is_pascal_case, is_snake_case, is_upper_snake_case,
    truncate_str,
};
use regex::Regex;
use std::collections::BTreeSet;

fn is_ignored_dir(dir: &str) -> bool {
    dir.starts_with('.')
        || matches!(
            dir.to_ascii_lowercase().as_str(),
            "target"
                | "build"
                | "dist"
                | "out"
                | "bin"
                | "obj"
                | "node_modules"
                | "venv"
                | "__pycache__"
                | "cmakefiles"
                | "cmake-build-debug"
                | "cmake-build-release"
                | "docs"
                | "doc"
                | "assets"
                | "images"
                | "img"
                | "static"
                | "public"
                | "vendor"
                | "third_party"
                | "pkg"
        )
}

fn is_project_marker_file(name: &str) -> bool {
    matches!(
        name,
        "run.sh"
            | "run.bat"
            | "Makefile"
            | "makefile"
            | "GNUmakefile"
            | "run_tests.sh"
            | "run_tests.bat"
            | "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "setup.py"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "CMakeLists.txt"
    )
}

pub fn detect_project_roots(input: &RepoInput) -> Vec<String> {
    let mut explicit_roots = BTreeSet::new();

    for f in &input.files {
        let segments: Vec<&str> = f.path.split('/').collect();
        if segments.is_empty() || is_ignored_dir(segments[0]) {
            continue;
        }

        for i in 0..segments.len() {
            if segments[..i].iter().any(|&s| {
                is_ignored_dir(s) || s.eq_ignore_ascii_case("src") || s.eq_ignore_ascii_case("tests") || s.eq_ignore_ascii_case("test")
            }) {
                break;
            }

            let seg = segments[i];
            let is_dir_marker = i + 1 < segments.len()
                && (seg.eq_ignore_ascii_case("src")
                    || seg.eq_ignore_ascii_case("tests")
                    || seg.eq_ignore_ascii_case("test"));
            let is_file_marker = i + 1 == segments.len() && is_project_marker_file(seg);

            if is_dir_marker || is_file_marker {
                let prefix = segments[..i].join("/");
                explicit_roots.insert(prefix);
            }
        }
    }

    let root_has_own_src = input.files.iter().any(|f| {
        f.path.starts_with("src/")
            || f.path.starts_with("tests/")
            || f.path.starts_with("test/")
    });

    if !root_has_own_src && explicit_roots.len() > 1 && explicit_roots.contains("") {
        explicit_roots.remove("");
    }

    if !explicit_roots.contains("") {
        let mut candidate_top_dirs = BTreeSet::new();
        for f in &input.files {
            let segments: Vec<&str> = f.path.split('/').collect();
            if segments.len() >= 2 {
                let top = segments[0];
                if !is_ignored_dir(top)
                    && !top.eq_ignore_ascii_case("src")
                    && !top.eq_ignore_ascii_case("tests")
                    && !top.eq_ignore_ascii_case("test")
                {
                    let is_code_or_readme = Lang::from_path(&f.path).is_some()
                        || segments
                            .last()
                            .map_or(false, |n| n.eq_ignore_ascii_case("readme.md"));
                    if is_code_or_readme {
                        candidate_top_dirs.insert(top.to_string());
                    }
                }
            }
        }

        for top_dir in candidate_top_dirs {
            let covered = explicit_roots
                .iter()
                .any(|r| r == &top_dir || r.starts_with(&format!("{}/", top_dir)));
            if !covered {
                explicit_roots.insert(top_dir);
            }
        }
    }

    if explicit_roots.is_empty() {
        vec![String::new()]
    } else {
        explicit_roots.into_iter().collect()
    }
}

pub fn check_structure(input: &RepoInput, violations: &mut Vec<Violation>) -> Vec<String> {
    let project_roots = detect_project_roots(input);

    let global_has_gitignore = input
        .files
        .iter()
        .any(|f| f.path == ".gitignore" || f.path.ends_with("/.gitignore"));
    let root_has_readme = input
        .files
        .iter()
        .any(|f| f.path.eq_ignore_ascii_case("readme.md"));
    let root_has_runner = input.files.iter().any(|f| {
        matches!(
            f.path.as_str(),
            "run.sh" | "run.bat" | "Makefile" | "makefile" | "GNUmakefile"
        )
    });

    if !global_has_gitignore {
        violations.push(make_violation(
            "С2",
            "Корень репозитория",
            None,
            None,
            "Файл .gitignore не найден ни в корне репозитория, ни в подпапках",
        ));
    }

    for proj in &project_roots {
        let scope_label = if proj.is_empty() {
            "Корень репозитория".to_string()
        } else {
            proj.clone()
        };

        let prefix = if proj.is_empty() {
            String::new()
        } else {
            format!("{}/", proj)
        };

        let has_local_readme = input.files.iter().any(|f| {
            f.path
                .strip_prefix(&prefix)
                .map_or(false, |rest| rest.eq_ignore_ascii_case("readme.md"))
        });
        if !has_local_readme && !root_has_readme {
            let expected = if proj.is_empty() {
                "README.md".to_string()
            } else {
                format!("{}/README.md", proj)
            };
            violations.push(make_violation(
                "С1",
                &scope_label,
                None,
                None,
                format!("Файл {} не найден", expected),
            ));
        }

        let has_src = input.files.iter().any(|f| {
            f.path.strip_prefix(&prefix).map_or(false, |rest| {
                let lower = rest.to_ascii_lowercase();
                lower.starts_with("src/") || lower.contains("/src/")
            })
        });
        if !has_src {
            let expected = if proj.is_empty() {
                "src/".to_string()
            } else {
                format!("{}/src/", proj)
            };
            violations.push(make_violation(
                "С3",
                &scope_label,
                None,
                None,
                format!("Директория {} отсутствует (некритично)", expected),
            ));
        }

        let has_tests = input.files.iter().any(|f| {
            f.path.strip_prefix(&prefix).map_or(false, |rest| {
                let lower = rest.to_ascii_lowercase();
                lower.starts_with("tests/")
                    || lower.starts_with("test/")
                    || lower.contains("/tests/")
                    || lower.contains("/test/")
            })
        });
        if !has_tests {
            let expected = if proj.is_empty() {
                "tests/".to_string()
            } else {
                format!("{}/tests/", proj)
            };
            violations.push(make_violation(
                "С4",
                &scope_label,
                None,
                None,
                format!("Директория {} отсутствует (некритично)", expected),
            ));
        }

        let has_local_runner = input.files.iter().any(|f| {
            f.path.strip_prefix(&prefix).map_or(false, |rest| {
                matches!(
                    rest,
                    "run.sh" | "run.bat" | "Makefile" | "makefile" | "GNUmakefile"
                )
            })
        });
        if !has_local_runner && !root_has_runner {
            let where_msg = if proj.is_empty() {
                "в корне репозитория".to_string()
            } else {
                format!("в папке {}", proj)
            };
            violations.push(make_violation(
                "С5",
                &scope_label,
                None,
                None,
                format!(
                    "Не найден ни один из файлов (run.sh, run.bat, Makefile) {}",
                    where_msg
                ),
            ));
        }
    }

    let conv_re = Regex::new(
        r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|init|release)(\([^)]+\))?!?: .+",
    )
    .unwrap();

    for commit in &input.commits {
        let first_line = commit.message.lines().next().unwrap_or("").trim();
        if first_line.starts_with("Merge ") || first_line.starts_with("Initial commit") {
            continue;
        }
        if !conv_re.is_match(first_line) {
            let short_sha = &commit.sha[..commit.sha.len().min(7)];
            violations.push(make_violation(
                "С6",
                format!("commit {}", short_sha),
                Some(1),
                None,
                first_line.to_string(),
            ));
        }
    }

    project_roots
}

pub fn check_generated_files(input: &RepoInput, violations: &mut Vec<Violation>) {
    let archive_exts = [
        ".zip", ".tar", ".gz", ".tgz", ".rar", ".7z", ".bz2", ".xz", ".zst", ".tar.gz",
    ];
    let binary_exts = [
        ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".obj", ".a", ".lib", ".class", ".wasm",
        ".pdb", ".jar", ".war", ".ear", ".elf", ".out",
    ];

    for file in &input.files {
        let lower = file.path.to_ascii_lowercase();
        let segments: Vec<&str> = file.path.split('/').collect();
        let filename = segments.last().copied().unwrap_or("");

        if archive_exts.iter().any(|ext| lower.ends_with(ext)) {
            violations.push(make_violation(
                "Г1",
                &file.path,
                None,
                None,
                "Архивный файл в репозитории",
            ));
        }

        let has_null = file.content.as_ref().map_or(false, |c| c.contains('\0'));
        if file.is_binary || has_null || binary_exts.iter().any(|ext| lower.ends_with(ext)) {
            violations.push(make_violation(
                "Г2",
                &file.path,
                None,
                None,
                "Бинарный файл в репозитории",
            ));
        }

        let is_py_service = segments.iter().any(|s| {
            matches!(
                *s,
                "__pycache__"
                    | ".pytest_cache"
                    | ".mypy_cache"
                    | ".ruff_cache"
                    | ".tox"
                    | ".venv"
                    | "venv"
                    | ".eggs"
            ) || s.ends_with(".egg-info")
        }) || lower.ends_with(".pyc")
            || lower.ends_with(".pyo")
            || lower.ends_with(".pyd")
            || filename == ".coverage";

        if is_py_service {
            violations.push(make_violation(
                "Г3",
                &file.path,
                None,
                None,
                "Служебный файл или директория Python",
            ));
        }

        let is_build_artifact = segments.iter().any(|s| {
            matches!(
                *s,
                "target"
                    | "build"
                    | "dist"
                    | "out"
                    | "bin"
                    | "obj"
                    | "node_modules"
                    | "CMakeFiles"
                    | "cmake-build-debug"
                    | "cmake-build-release"
                    | ".next"
                    | ".nuxt"
                    | ".gradle"
            )
        }) || filename == "CMakeCache.txt";

        if is_build_artifact {
            violations.push(make_violation("Г4", &file.path, None, None, "Артефакт сборки"));
        }

        let is_editor_config = segments.iter().any(|s| {
            matches!(
                *s,
                ".vscode" | ".idea" | ".vs" | ".fleet" | ".eclipse" | ".settings"
            )
        }) || matches!(filename, ".project" | ".classpath" | ".DS_Store")
            || lower.ends_with(".suo")
            || lower.ends_with(".user")
            || lower.ends_with(".swp")
            || lower.ends_with(".swo")
            || lower.ends_with(".iml");

        if is_editor_config {
            violations.push(make_violation(
                "Г5",
                &file.path,
                None,
                None,
                "Конфигурационный файл IDE / редактора кода",
            ));
        }
    }
}

pub fn check_review_rules(
    path: &str,
    raw_lines: &[&str],
    lang: Option<Lang>,
    violations: &mut Vec<Violation>,
) {
    if raw_lines.len() > 1000 {
        violations.push(make_violation(
            "Р1",
            path,
            Some(1001),
            Some(raw_lines.len()),
            format!("Размер файла: {} строк (максимум 1000)", raw_lines.len()),
        ));
    }

    for (idx, line) in raw_lines.iter().enumerate() {
        let line_no = idx + 1;
        let len = line.chars().count();

        match lang {
            Some(Lang::Python) if len > 80 => {
                violations.push(make_violation(
                    "Р2",
                    path,
                    Some(line_no),
                    None,
                    format!("Длина строки {} символов (>80): {}", len, truncate_str(line, 60)),
                ));
            }
            Some(Lang::JavaScript) if len > 80 => {
                violations.push(make_violation(
                    "Р3",
                    path,
                    Some(line_no),
                    None,
                    format!("Длина строки {} символов (>80): {}", len, truncate_str(line, 60)),
                ));
            }
            Some(Lang::Kotlin) if len > 100 => {
                violations.push(make_violation(
                    "Р4",
                    path,
                    Some(line_no),
                    None,
                    format!("Длина строки {} символов (>100): {}", len, truncate_str(line, 60)),
                ));
            }
            Some(Lang::Rust) if len > 100 => {
                violations.push(make_violation(
                    "Р5",
                    path,
                    Some(line_no),
                    None,
                    format!("Длина строки {} символов (>100): {}", len, truncate_str(line, 60)),
                ));
            }
            _ => {}
        }

        if lang.is_some() && len > 120 {
            violations.push(make_violation(
                "Р6",
                path,
                Some(line_no),
                None,
                format!("Длина строки {} символов (>120): {}", len, truncate_str(line, 60)),
            ));
        }
    }
}

pub fn check_functions_and_args(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    clean_lines: &[String],
    violations: &mut Vec<Violation>,
) {
    let (f_rule, a_rule, c_rule) = match lang {
        Lang::Python => ("Ф1", "А1", "Ц1"),
        Lang::JavaScript => ("Ф2", "А2", "Ц2"),
        Lang::CSharp => ("Ф3", "А3", "Ц3"),
        Lang::Java => ("Ф4", "А4", "Ц4"),
        Lang::Go => ("Ф5", "А5", "Ц5"),
        Lang::Kotlin => ("Ф6", "А6", "Ц6"),
        Lang::Rust => ("Ф7", "А7", "Ц7"),
        Lang::OtherCode => return,
    };

    for func in funcs {
        let func_len = func.end_line.saturating_sub(func.start_line) + 1;
        if func_len > 40 {
            violations.push(make_violation(
                f_rule,
                path,
                Some(func.start_line),
                Some(func.end_line),
                format!(
                    "Функция `{}` занимает {} строк (максимум 40)",
                    func.name, func_len
                ),
            ));
        }

        if func.arg_count > 7 {
            violations.push(make_violation(
                a_rule,
                path,
                Some(func.start_line),
                None,
                format!(
                    "Функция `{}` принимает {} аргументов (максимум 7)",
                    func.name, func.arg_count
                ),
            ));
        }

        let body_slice = &clean_lines[func.start_line - 1..func.end_line.min(clean_lines.len())];
        let complexity = calculate_cyclomatic_complexity(lang, body_slice);
        if complexity > 10 {
            violations.push(make_violation(
                c_rule,
                path,
                Some(func.start_line),
                Some(func.end_line),
                format!(
                    "Функция `{}` имеет цикломатическую сложность {} (максимум 10)",
                    func.name, complexity
                ),
            ));
        }
    }
}

fn calculate_cyclomatic_complexity(lang: Lang, clean_body_lines: &[String]) -> usize {
    let kw_re = match lang {
        Lang::Python => Regex::new(r"\b(if|elif|for|while|except|and|or|case)\b").unwrap(),
        Lang::JavaScript => Regex::new(r"\b(if|for|while|case|catch)\b|&&|\|\||\?\?").unwrap(),
        Lang::CSharp => {
            Regex::new(r"\b(if|for|foreach|while|case|catch)\b|&&|\|\||\?\?").unwrap()
        }
        Lang::Java => Regex::new(r"\b(if|for|while|case|catch)\b|&&|\|\|").unwrap(),
        Lang::Go => Regex::new(r"\b(if|for|case)\b|&&|\|\|").unwrap(),
        Lang::Kotlin => Regex::new(r"\b(if|for|while|catch)\b|&&|\|\||\?:|->").unwrap(),
        Lang::Rust => Regex::new(r"\b(if|for|while|loop)\b|&&|\|\||=>").unwrap(),
        Lang::OtherCode => return 1,
    };

    let mut complexity = 1usize;
    for clean in clean_body_lines {
        complexity += kw_re.find_iter(clean).count();
    }
    complexity
}

pub fn check_comments_vs_docs(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    raw_lines: &[&str],
    violations: &mut Vec<Violation>,
) {
    let rule_id = match lang {
        Lang::Python => "К1",
        Lang::JavaScript => "К2",
        Lang::CSharp => "К3",
        Lang::Java => "К4",
        Lang::Go => "К5",
        Lang::Kotlin => "К6",
        Lang::Rust => "К7",
        Lang::OtherCode => return,
    };

    for func in funcs {
        let def_idx = func.start_line - 1;

        if lang == Lang::Python {
            if def_idx > 0 {
                let prev = raw_lines[def_idx - 1].trim();
                if prev.starts_with('#') && !prev.starts_with("#!") {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(def_idx),
                        None,
                        format!(
                            "Комментарий `{}` перед `{}` вместо docstring",
                            prev, func.name
                        ),
                    ));
                }
            }
            if def_idx + 1 < raw_lines.len() {
                let first_body = raw_lines[def_idx + 1].trim();
                if first_body.starts_with('#') {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(def_idx + 2),
                        None,
                        format!(
                            "Комментарий `{}` внутри `{}` вместо docstring",
                            first_body, func.name
                        ),
                    ));
                }
            }
            continue;
        }

        let mut check_idx = def_idx;
        while check_idx > 0 {
            let prev = raw_lines[check_idx - 1].trim();
            if prev.starts_with('@') || prev.starts_with("#[") || prev.starts_with('[') {
                check_idx -= 1;
                continue;
            }
            break;
        }

        if check_idx == 0 {
            continue;
        }

        let prev_line = raw_lines[check_idx - 1].trim();
        let comment_line_no = check_idx;

        match lang {
            Lang::JavaScript | Lang::Java | Lang::Kotlin => {
                if prev_line.starts_with("//")
                    || (prev_line.starts_with("/*") && !prev_line.starts_with("/**"))
                {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(comment_line_no),
                        None,
                        format!(
                            "Обычный комментарий перед `{}` вместо документационного /** ... */",
                            func.name
                        ),
                    ));
                }
            }
            Lang::CSharp => {
                if (prev_line.starts_with("//") && !prev_line.starts_with("///"))
                    || (prev_line.starts_with("/*") && !prev_line.starts_with("/**"))
                {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(comment_line_no),
                        None,
                        format!(
                            "Обычный комментарий перед `{}` вместо XMLDoc `///`",
                            func.name
                        ),
                    ));
                }
            }
            Lang::Go => {
                if prev_line.ends_with("*/") || prev_line.starts_with("/*") {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(comment_line_no),
                        None,
                        format!(
                            "Блочный комментарий перед `{}` вместо стандартного Godoc `// {} ...`",
                            func.name, func.name
                        ),
                    ));
                } else if prev_line.starts_with("//") {
                    let mut top_comment_idx = check_idx - 1;
                    while top_comment_idx > 0
                        && raw_lines[top_comment_idx - 1].trim().starts_with("//")
                    {
                        top_comment_idx -= 1;
                    }
                    let first_comment = raw_lines[top_comment_idx]
                        .trim()
                        .trim_start_matches("//")
                        .trim();
                    if !first_comment.starts_with(&func.name) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(top_comment_idx + 1),
                            None,
                            format!(
                                "Комментарий перед `{}` не начинается с имени функции по стандарту Godoc",
                                func.name
                            ),
                        ));
                    }
                }
            }
            Lang::Rust => {
                if (prev_line.starts_with("//")
                    && !prev_line.starts_with("///")
                    && !prev_line.starts_with("//!"))
                    || (prev_line.starts_with("/*")
                        && !prev_line.starts_with("/**")
                        && !prev_line.starts_with("/*!"))
                {
                    violations.push(make_violation(
                        rule_id,
                        path,
                        Some(comment_line_no),
                        None,
                        format!(
                            "Обычный комментарий перед `{}` вместо rustdoc `///`",
                            func.name
                        ),
                    ));
                }
            }
            _ => {}
        }
    }
}

pub fn check_magic_numbers(
    path: &str,
    lang: Lang,
    raw_lines: &[&str],
    clean_lines: &[String],
    violations: &mut Vec<Violation>,
) {
    let rule_id = match lang {
        Lang::Python => "М1",
        Lang::JavaScript => "М2",
        Lang::CSharp => "М3",
        Lang::Java => "М4",
        Lang::Go => "М5",
        Lang::Kotlin => "М6",
        Lang::Rust => "М7",
        Lang::OtherCode => return,
    };

    let semver_re = Regex::new(r"\b\d+\.\d+(?:\.\d+)+\b").unwrap();
    let generic_re = Regex::new(r"<[A-Za-z0-9_,\s\?]+>").unwrap();
    let cmp_right_re = Regex::new(
        r"[A-Za-z0-9_)\]]\s+(?:===|!==|==|!=|<=|>=|<|>)\s*(-?\d+(?:\.\d+)?)\b",
    )
    .unwrap();
    let cmp_left_re = Regex::new(
        r"\b(-?\d+(?:\.\d+)?)\s*(?:===|!==|==|!=|<=|>=|<|>)\s+[A-Za-z_]",
    )
    .unwrap();

    for (idx, clean_line) in clean_lines.iter().enumerate() {
        let trimmed_clean = clean_line.trim();
        if trimmed_clean.is_empty()
            || trimmed_clean.starts_with("const ")
            || trimmed_clean.starts_with("pub const ")
            || trimmed_clean.starts_with("private const ")
            || trimmed_clean.starts_with("public const ")
            || trimmed_clean.contains("static final ")
            || trimmed_clean.starts_with("const val ")
        {
            continue;
        }

        let no_versions = semver_re.replace_all(trimmed_clean, " ");
        let normalized = no_versions
            .replace("->", "  ")
            .replace("=>", "  ")
            .replace("<<", "  ")
            .replace(">>", "  ");
        let normalized = generic_re.replace_all(&normalized, " ");

        let spaced = normalized
            .replace("===", " === ")
            .replace("!==", " !== ")
            .replace("==", " == ")
            .replace("!=", " != ")
            .replace("<=", " <= ")
            .replace(">=", " >= ");

        let mut found_magic = None;

        for caps in cmp_right_re.captures_iter(&spaced) {
            let m = caps.get(1).unwrap();
            let after = &spaced[m.end()..];
            if after.starts_with('.') {
                continue;
            }
            let num = m.as_str();
            if !is_allowed_number(num) {
                found_magic = Some(num.to_string());
                break;
            }
        }

        if found_magic.is_none() {
            for caps in cmp_left_re.captures_iter(&spaced) {
                let m = caps.get(1).unwrap();
                let before = &spaced[..m.start()];
                if before.ends_with('.') {
                    continue;
                }
                let num = m.as_str();
                if !is_allowed_number(num) {
                    found_magic = Some(num.to_string());
                    break;
                }
            }
        }

        if let Some(magic) = found_magic {
            let raw_snippet = raw_lines.get(idx).copied().unwrap_or("").trim();
            violations.push(make_violation(
                rule_id,
                path,
                Some(idx + 1),
                None,
                format!("Магическое число `{}` в сравнении: {}", magic, raw_snippet),
            ));
        }
    }
}

fn is_allowed_number(num: &str) -> bool {
    matches!(num, "0" | "1" | "-1" | "0.0" | "1.0" | "-1.0")
}

pub fn check_naming_conventions(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    clean_lines: &[String],
    violations: &mut Vec<Violation>,
) {
    let rule_id = match lang {
        Lang::Python => "И1",
        Lang::JavaScript => "И2",
        Lang::CSharp => "И3",
        Lang::Java => "И4",
        Lang::Go => "И5",
        Lang::Kotlin => "И6",
        Lang::Rust => "И7",
        Lang::OtherCode => return,
    };

    for func in funcs {
        let valid_func = match lang {
            Lang::Python | Lang::Rust => is_snake_case(&func.name),
            Lang::JavaScript => is_camel_case(&func.name) || is_pascal_case(&func.name),
            Lang::CSharp => is_pascal_case(&func.name),
            Lang::Java | Lang::Kotlin => is_camel_case(&func.name),
            Lang::Go => is_go_valid_name(&func.name, path.ends_with("_test.go")),
            Lang::OtherCode => true,
        };

        if !valid_func {
            violations.push(make_violation(
                rule_id,
                path,
                Some(func.start_line),
                None,
                format!(
                    "Имя функции/метода `{}` нарушает соглашение об именовании",
                    func.name
                ),
            ));
        }

        for (param_name, p_line) in &func.params_raw {
            if param_name == "_" {
                continue;
            }
            let valid_param = match lang {
                Lang::Python | Lang::Rust => is_snake_case(param_name),
                Lang::JavaScript | Lang::CSharp | Lang::Java | Lang::Kotlin => {
                    is_camel_case(param_name)
                }
                Lang::Go => is_go_valid_name(param_name, false),
                Lang::OtherCode => true,
            };
            if !valid_param {
                violations.push(make_violation(
                    rule_id,
                    path,
                    Some(*p_line),
                    None,
                    format!(
                        "Имя параметра `{}` нарушает соглашение об именовании",
                        param_name
                    ),
                ));
            }
        }
    }

    let type_re = Regex::new(
        r"^\s*(?:pub(?:\([^)]+\))?\s+|public\s+|private\s+|protected\s+|internal\s+|export\s+|abstract\s+|final\s+|sealed\s+|data\s+)*(?:class|struct|enum|interface|trait)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
    let go_type_re = Regex::new(r"^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+").unwrap();
    let py_var_re = Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*[^=]").unwrap();
    let js_var_re =
        Regex::new(r"^\s*(?:let|const|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=").unwrap();
    let rust_let_re =
        Regex::new(r"^\s*let\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*(?::|=)").unwrap();
    let rust_const_re =
        Regex::new(r"^\s*(?:pub\s+)?(?:const|static)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:").unwrap();
    let kt_var_re = Regex::new(r"^\s*(?:val|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?::|=)").unwrap();
    let go_var_re =
        Regex::new(r"^\s*(?:var|const)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:=|[A-Za-z_\[\]*])")
            .unwrap();

    for (idx, clean) in clean_lines.iter().enumerate() {
        let line_no = idx + 1;

        if let Some(caps) = type_re.captures(clean) {
            let type_name = &caps[1];
            if !is_pascal_case(type_name) {
                violations.push(make_violation(
                    rule_id,
                    path,
                    Some(line_no),
                    None,
                    format!("Имя типа `{}` должно быть в PascalCase", type_name),
                ));
            }
        }

        match lang {
            Lang::Python => {
                if let Some(caps) = py_var_re.captures(clean) {
                    let var_name = &caps[1];
                    if !is_snake_case(var_name) && !is_upper_snake_case(var_name) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!(
                                "Имя переменной `{}` не соответствует PEP8 (snake_case)",
                                var_name
                            ),
                        ));
                    }
                }
            }
            Lang::JavaScript => {
                if let Some(caps) = js_var_re.captures(clean) {
                    let var_name = &caps[1];
                    if !is_camel_case(var_name)
                        && !is_upper_snake_case(var_name)
                        && !is_pascal_case(var_name)
                    {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!(
                                "Имя идентификатора `{}` нарушает ESLint camelCase",
                                var_name
                            ),
                        ));
                    }
                }
            }
            Lang::Go => {
                if let Some(caps) = go_type_re.captures(clean) {
                    let t_name = &caps[1];
                    if !is_go_valid_name(t_name, false) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!(
                                "Имя типа Go `{}` содержит подчеркивания (не MixedCaps)",
                                t_name
                            ),
                        ));
                    }
                }
                if let Some(caps) = go_var_re.captures(clean) {
                    let v_name = &caps[1];
                    if v_name != "_" && !is_go_valid_name(v_name, false) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!("Имя идентификатора Go `{}` содержит подчеркивания", v_name),
                        ));
                    }
                }
            }
            Lang::Kotlin => {
                if let Some(caps) = kt_var_re.captures(clean) {
                    let v_name = &caps[1];
                    if !is_camel_case(v_name) && !is_upper_snake_case(v_name) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!(
                                "Имя свойства/переменной Kotlin `{}` должно быть в camelCase",
                                v_name
                            ),
                        ));
                    }
                }
            }
            Lang::Rust => {
                if let Some(caps) = rust_let_re.captures(clean) {
                    let v_name = &caps[1];
                    if !is_snake_case(v_name) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!("Имя переменной Rust `{}` должно быть в snake_case", v_name),
                        ));
                    }
                }
                if let Some(caps) = rust_const_re.captures(clean) {
                    let c_name = &caps[1];
                    if !is_upper_snake_case(c_name) {
                        violations.push(make_violation(
                            rule_id,
                            path,
                            Some(line_no),
                            None,
                            format!(
                                "Имя константы Rust `{}` должно быть в UPPER_SNAKE_CASE",
                                c_name
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Debug)]
pub struct RepoInput {
    pub repo_name: String,
    pub files: Vec<FileEntry>,
    pub commits: Vec<CommitEntry>,
}

#[derive(Deserialize, Debug)]
pub struct FileEntry {
    pub path: String,
    pub content: Option<String>,
    #[serde(default)]
    pub is_binary: bool,
}

#[derive(Deserialize, Debug)]
pub struct CommitEntry {
    pub sha: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Violation {
    pub rule_id: String,
    pub category: String,
    pub description: String,
    pub file: String,
    pub line: Option<usize>,
    pub end_line: Option<usize>,
    pub snippet: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct RuleStatus {
    pub rule_id: String,
    pub category: String,
    pub description: String,
    pub count: usize,
    pub passed: bool,
}

#[derive(Serialize, Debug)]
pub struct AnalysisReport {
    pub repo_name: String,
    pub detected_projects: Vec<String>,
    pub total_files: usize,
    pub total_commits: usize,
    pub total_violations: usize,
    pub passed_rules: usize,
    pub failed_rules: usize,
    pub rules_summary: Vec<RuleStatus>,
    pub violations: Vec<Violation>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    Python,
    JavaScript,
    CSharp,
    Java,
    Go,
    Kotlin,
    Rust,
    OtherCode,
}

impl Lang {
    fn from_path(path: &str) -> Option<Self> {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".py") {
            Some(Lang::Python)
        } else if lower.ends_with(".js")
            || lower.ends_with(".jsx")
            || lower.ends_with(".mjs")
            || lower.ends_with(".cjs")
        {
            Some(Lang::JavaScript)
        } else if lower.ends_with(".cs") {
            Some(Lang::CSharp)
        } else if lower.ends_with(".java") {
            Some(Lang::Java)
        } else if lower.ends_with(".go") {
            Some(Lang::Go)
        } else if lower.ends_with(".kt") || lower.ends_with(".kts") {
            Some(Lang::Kotlin)
        } else if lower.ends_with(".rs") {
            Some(Lang::Rust)
        } else if lower.ends_with(".c")
            || lower.ends_with(".cpp")
            || lower.ends_with(".h")
            || lower.ends_with(".hpp")
            || lower.ends_with(".ts")
            || lower.ends_with(".tsx")
            || lower.ends_with(".php")
            || lower.ends_with(".rb")
            || lower.ends_with(".swift")
        {
            Some(Lang::OtherCode)
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct ParsedFunction {
    name: String,
    start_line: usize,
    end_line: usize,
    arg_count: usize,
    params_raw: Vec<(String, usize)>,
}

struct RuleCatalogItem {
    id: &'static str,
    category: &'static str,
    desc: &'static str,
}

const ALL_RULES: &[RuleCatalogItem] = &[
    // Структура репозитория
    RuleCatalogItem { id: "С1", category: "Структура репозитория", desc: "В репозитории отсутствует файл README.md" },
    RuleCatalogItem { id: "С2", category: "Структура репозитория", desc: "В репозитории отсутствует файл .gitignore" },
    RuleCatalogItem { id: "С3", category: "Структура репозитория", desc: "В репозитории отсутствует папка с исходным кодом src" },
    RuleCatalogItem { id: "С4", category: "Структура репозитория", desc: "В репозитории отсутствует папка с тестами tests" },
    RuleCatalogItem { id: "С5", category: "Структура репозитория", desc: "В репозитории отсутствует скрипт запуска run.sh, run.bat или Makefile" },
    RuleCatalogItem { id: "С6", category: "Структура репозитория", desc: "Коммиты не соответствуют спецификации Conventional/Scoped Commits" },
    // Сгенерированные файлы
    RuleCatalogItem { id: "Г1", category: "Сгенерированные файлы", desc: "Репозиторий содержит архивы" },
    RuleCatalogItem { id: "Г2", category: "Сгенерированные файлы", desc: "Репозиторий содержит бинарные файлы" },
    RuleCatalogItem { id: "Г3", category: "Сгенерированные файлы", desc: "Репозиторий содержит служебные файлы языка Python" },
    RuleCatalogItem { id: "Г4", category: "Сгенерированные файлы", desc: "Репозиторий содержит артефакты сборки" },
    RuleCatalogItem { id: "Г5", category: "Сгенерированные файлы", desc: "Репозиторий содержит файлы с настройками редактора кода" },
    // Рецензирование
    RuleCatalogItem { id: "Р1", category: "Рецензирование", desc: "Репозиторий содержит слишком большие файлы (>1000 строк)" },
    RuleCatalogItem { id: "Р2", category: "Рецензирование", desc: "Код на языке Python содержит слишком длинные строки (>80 символов)" },
    RuleCatalogItem { id: "Р3", category: "Рецензирование", desc: "Код на языке JavaScript содержит слишком длинные строки (>80 символов)" },
    RuleCatalogItem { id: "Р4", category: "Рецензирование", desc: "Код на языке Kotlin содержит слишком длинные строки (>100 символов)" },
    RuleCatalogItem { id: "Р5", category: "Рецензирование", desc: "Код на языке Rust содержит слишком длинные строки (>100 символов)" },
    RuleCatalogItem { id: "Р6", category: "Рецензирование", desc: "Код в репозитории содержит слишком длинные строки (>120 символов)" },
    // Функции
    RuleCatalogItem { id: "Ф1", category: "Функции", desc: "Код на языке Python содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф2", category: "Функции", desc: "Код на языке JavaScript содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф3", category: "Функции", desc: "Код на языке C# содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф4", category: "Функции", desc: "Код на языке Java содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф5", category: "Функции", desc: "Код на языке Go содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф6", category: "Функции", desc: "Код на языке Kotlin содержит слишком длинные функции (>40 строк)" },
    RuleCatalogItem { id: "Ф7", category: "Функции", desc: "Код на языке Rust содержит слишком длинные функции (>40 строк)" },
    // Аргументы функций
    RuleCatalogItem { id: "А1", category: "Аргументы функций", desc: "Код на Python содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А2", category: "Аргументы функций", desc: "Код на JavaScript содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А3", category: "Аргументы функций", desc: "Код на C# содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А4", category: "Аргументы функций", desc: "Код на Java содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А5", category: "Аргументы функций", desc: "Код на Go содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А6", category: "Аргументы функций", desc: "Код на Kotlin содержит функции с большим числом аргументов (>7)" },
    RuleCatalogItem { id: "А7", category: "Аргументы функций", desc: "Код на Rust содержит функции с большим числом аргументов (>7)" },
    // Комментарии вместо документации
    RuleCatalogItem { id: "К1", category: "Комментарии вместо документации", desc: "Код на Python содержит комментарии вместо документации docstring" },
    RuleCatalogItem { id: "К2", category: "Комментарии вместо документации", desc: "Код на JavaScript содержит комментарии вместо документации JSDoc" },
    RuleCatalogItem { id: "К3", category: "Комментарии вместо документации", desc: "Код на C# содержит комментарии вместо документации XMLDoc" },
    RuleCatalogItem { id: "К4", category: "Комментарии вместо документации", desc: "Код на Java содержит комментарии вместо документации Javadoc" },
    RuleCatalogItem { id: "К5", category: "Комментарии вместо документации", desc: "Код на Go содержит комментарии вместо документации Godoc" },
    RuleCatalogItem { id: "К6", category: "Комментарии вместо документации", desc: "Код на Kotlin содержит комментарии вместо документации KDoc" },
    RuleCatalogItem { id: "К7", category: "Комментарии вместо документации", desc: "Код на Rust содержит комментарии вместо документации rustdoc" },
    // Цикломатическая сложность кода
    RuleCatalogItem { id: "Ц1", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Python превышена (>10)" },
    RuleCatalogItem { id: "Ц2", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке JavaScript превышена (>10)" },
    RuleCatalogItem { id: "Ц3", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке C# превышена (>10)" },
    RuleCatalogItem { id: "Ц4", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Java превышена (>10)" },
    RuleCatalogItem { id: "Ц5", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Go превышена (>10)" },
    RuleCatalogItem { id: "Ц6", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Kotlin превышена (>10)" },
    RuleCatalogItem { id: "Ц7", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Rust превышена (>10)" },
    // Магические числа
    RuleCatalogItem { id: "М1", category: "Магические числа", desc: "Код на Python содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М2", category: "Магические числа", desc: "Код на JavaScript содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М3", category: "Магические числа", desc: "Код на C# содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М4", category: "Магические числа", desc: "Код на Java содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М5", category: "Магические числа", desc: "Код на Go содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М6", category: "Магические числа", desc: "Код на Kotlin содержит магические числовые литералы в сравнениях" },
    RuleCatalogItem { id: "М7", category: "Магические числа", desc: "Код на Rust содержит магические числовые литералы в сравнениях" },
    // Имена идентификаторов
    RuleCatalogItem { id: "И1", category: "Имена идентификаторов", desc: "Код на Python содержит имена, не соответствующие стандарту PEP8" },
    RuleCatalogItem { id: "И2", category: "Имена идентификаторов", desc: "Код на JavaScript содержит имена, нарушающие соглашения ESLint" },
    RuleCatalogItem { id: "И3", category: "Имена идентификаторов", desc: "Код на C# содержит имена, нарушающие соглашения .NET Runtime" },
    RuleCatalogItem { id: "И4", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Java" },
    RuleCatalogItem { id: "И5", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Go" },
    RuleCatalogItem { id: "И6", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Kotlin" },
    RuleCatalogItem { id: "И7", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Rust" },
];

fn make_violation(
    rule_id: &str,
    file: impl Into<String>,
    line: Option<usize>,
    end_line: Option<usize>,
    snippet: impl Into<String>,
) -> Violation {
    let item = ALL_RULES
        .iter()
        .find(|r| r.id == rule_id)
        .expect("Unknown rule ID");
    Violation {
        rule_id: item.id.to_string(),
        category: item.category.to_string(),
        description: item.desc.to_string(),
        file: file.into(),
        line,
        end_line,
        snippet: snippet.into(),
    }
}

#[wasm_bindgen]
pub fn analyze_repository(input_json: &str) -> String {
    let input: RepoInput = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(e) => {
            return format!(
                "{{\"error\": \"Ошибка десериализации входных данных: {}\"}}",
                e
            )
        }
    };

    let report = run_all_checks(&input);
    serde_json::to_string(&report)
        .unwrap_or_else(|_| "{\"error\": \"Ошибка сериализации отчета\"}".to_string())
}

pub fn run_all_checks(input: &RepoInput) -> AnalysisReport {
    let mut violations = Vec::new();

    let detected_projects = check_structure(input, &mut violations);
    check_generated_files(input, &mut violations);

    for file in &input.files {
        let Some(content) = &file.content else {
            continue;
        };
        if file.is_binary || content.contains('\0') {
            continue;
        }

        let lines: Vec<&str> = content.lines().collect();
        let lang = Lang::from_path(&file.path);

        // Р1-Р6: Длина файлов и строк
        check_review_rules(&file.path, &lines, lang, &mut violations);

        if let Some(l) = lang {
            if l != Lang::OtherCode {
                let funcs = extract_functions(l, &lines);
                check_functions_and_args(&file.path, l, &funcs, &lines, &mut violations);
                check_comments_vs_docs(&file.path, l, &funcs, &lines, &mut violations);
                check_magic_numbers(&file.path, l, &lines, &mut violations);
                check_naming_conventions(&file.path, l, &funcs, &lines, &mut violations);
            }
        }
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for v in &violations {
        if let Some(rule) = ALL_RULES.iter().find(|r| r.id == v.rule_id) {
            *counts.entry(rule.id).or_insert(0) += 1;
        }
    }

    let mut rules_summary = Vec::with_capacity(ALL_RULES.len());
    let mut passed_rules = 0;
    let mut failed_rules = 0;

    for r in ALL_RULES {
        let count = *counts.get(r.id).unwrap_or(&0);
        if count == 0 {
            passed_rules += 1;
        } else {
            failed_rules += 1;
        }
        rules_summary.push(RuleStatus {
            rule_id: r.id.to_string(),
            category: r.category.to_string(),
            description: r.desc.to_string(),
            count,
            passed: count == 0,
        });
    }

    let project_labels = detected_projects
        .into_iter()
        .map(|p| if p.is_empty() { "/ (корень)".to_string() } else { p })
        .collect();

    AnalysisReport {
        repo_name: input.repo_name.clone(),
        detected_projects: project_labels,
        total_files: input.files.len(),
        total_commits: input.commits.len(),
        total_violations: violations.len(),
        passed_rules,
        failed_rules,
        rules_summary,
        violations,
    }
}

// ============================================================================
// 1. СТРУКТУРА РЕПОЗИТОРИЯ С ПОДДЕРЖКОЙ ПОДПАПОК ПРАКТИК (С1 - С6)
// ============================================================================
fn is_ignored_top_dir(dir: &str) -> bool {
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
                | "docs"
                | "doc"
                | "assets"
                | "images"
                | "img"
                | "static"
                | "public"
                | "vendor"
                | "third_party"
        )
}

fn is_runner_filename(name: &str) -> bool {
    matches!(
        name,
        "run.sh"
            | "run.bat"
            | "Makefile"
            | "makefile"
            | "GNUmakefile"
            | "run_tests.sh"
            | "run_tests.bat"
    )
}

/// Определяет директории проектов внутри репозитория.
/// Возвращает `vec![""]`, если проект лежит прямо в корне,
/// либо список подпапок практик (например, `vec!["PR_1", "PR_2"]`).
fn detect_project_roots(input: &RepoInput) -> Vec<String> {
    let mut explicit_roots = BTreeSet::new();

    for f in &input.files {
        let segments: Vec<&str> = f.path.split('/').collect();
        if segments.is_empty() {
            continue;
        }
        if is_ignored_top_dir(segments[0]) {
            continue;
        }

        // Ищем папки, внутри которых лежат src/, tests/ или скрипты запуска
        for i in 0..segments.len() {
            let seg = segments[i];
            let is_structure_marker = (i + 1 < segments.len() && (seg == "src" || seg == "tests"))
                || (i + 1 == segments.len() && is_runner_filename(seg));

            if is_structure_marker {
                let prefix = segments[..i].join("/");
                // Исключаем вложенные пути внутри уже найденных src/ или tests/
                if !segments[..i].iter().any(|&s| s == "src" || s == "tests") {
                    explicit_roots.insert(prefix);
                }
            }
        }
    }

    // Если в подпапках найдены проекты (например, PR_1, PR_2), а также есть какие-то маркеры в корне,
    // приоритет отдается подпапкам практик, если в самом корне нет собственной папки src/
    let root_has_own_src = input.files.iter().any(|f| f.path.starts_with("src/"));
    if !root_has_own_src && explicit_roots.len() > 1 && explicit_roots.contains("") {
        explicit_roots.remove("");
    }

    // Если проект не в корне, проверим также верхнеуровневые папки с кодом,
    // где студент мог вообще забыть создать и src/, и tests/, и run.sh
    if !explicit_roots.contains("") {
        let mut top_level_code_dirs = BTreeSet::new();
        for f in &input.files {
            let segments: Vec<&str> = f.path.split('/').collect();
            if segments.len() >= 2 && !is_ignored_top_dir(segments[0]) {
                if Lang::from_path(&f.path).is_some()
                    || segments.last().map_or(false, |name| name.eq_ignore_ascii_case("readme.md"))
                {
                    top_level_code_dirs.insert(segments[0].to_string());
                }
            }
        }

        for top_dir in top_level_code_dirs {
            let already_covered = explicit_roots
                .iter()
                .any(|r| r == &top_dir || r.starts_with(&format!("{}/", top_dir)));
            if !already_covered {
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

fn check_structure(input: &RepoInput, violations: &mut Vec<Violation>) -> Vec<String> {
    let project_roots = detect_project_roots(input);

    // Глобальные маркеры в корне репозитория (например, общий .gitignore или общий README.md)
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

    // С2: .gitignore должен быть хотя бы в корне репозитория (или в папках практик)
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

        // С1: README.md внутри папки практики (или в корне репозитория)
        let has_local_readme = input.files.iter().any(|f| {
            if let Some(rest) = f.path.strip_prefix(&prefix) {
                rest.eq_ignore_ascii_case("readme.md")
            } else {
                false
            }
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

        // С3: Папка src внутри практики
        let has_src = input.files.iter().any(|f| {
            if let Some(rest) = f.path.strip_prefix(&prefix) {
                rest.starts_with("src/") || rest.contains("/src/")
            } else {
                false
            }
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
                format!("Директория {} отсутствует", expected),
            ));
        }

        // С4: Папка tests внутри практики
        let has_tests = input.files.iter().any(|f| {
            if let Some(rest) = f.path.strip_prefix(&prefix) {
                rest.starts_with("tests/") || rest.contains("/tests/")
            } else {
                false
            }
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
                format!("Директория {} отсутствует", expected),
            ));
        }

        // С5: Скрипт запуска run.sh, run.bat или Makefile внутри практики (или в корне)
        let has_local_runner = input.files.iter().any(|f| {
            if let Some(rest) = f.path.strip_prefix(&prefix) {
                matches!(
                    rest,
                    "run.sh" | "run.bat" | "Makefile" | "makefile" | "GNUmakefile"
                )
            } else {
                false
            }
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

    // С6: Conventional / Scoped Commits
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

// ============================================================================
// 2. СГЕНЕРИРОВАННЫЕ ФАЙЛЫ (Г1 - Г5)
// ============================================================================
fn check_generated_files(input: &RepoInput, violations: &mut Vec<Violation>) {
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

        // Г1: Архивы
        if archive_exts.iter().any(|ext| lower.ends_with(ext)) {
            violations.push(make_violation(
                "Г1",
                &file.path,
                None,
                None,
                "Архивный файл в репозитории",
            ));
        }

        // Г2: Бинарные файлы
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

        // Г3: Служебные файлы Python
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

        // Г4: Артефакты сборки
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

        // Г5: Файлы с настройками редактора кода
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

// ============================================================================
// 3. РЕЦЕНЗИРОВАНИЕ (Р1 - Р6)
// ============================================================================
fn check_review_rules(
    path: &str,
    lines: &[&str],
    lang: Option<Lang>,
    violations: &mut Vec<Violation>,
) {
    if lines.len() > 1000 {
        violations.push(make_violation(
            "Р1",
            path,
            Some(1001),
            Some(lines.len()),
            format!("Размер файла: {} строк (максимум 1000)", lines.len()),
        ));
    }

    for (idx, line) in lines.iter().enumerate() {
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
                    format!(
                        "Длина строки {} символов (>100): {}",
                        len,
                        truncate_str(line, 60)
                    ),
                ));
            }
            Some(Lang::Rust) if len > 100 => {
                violations.push(make_violation(
                    "Р5",
                    path,
                    Some(line_no),
                    None,
                    format!(
                        "Длина строки {} символов (>100): {}",
                        len,
                        truncate_str(line, 60)
                    ),
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
                format!(
                    "Длина строки {} символов (>120): {}",
                    len,
                    truncate_str(line, 60)
                ),
            ));
        }
    }
}

// ============================================================================
// ПАРСЕР ФУНКЦИЙ ДЛЯ 7 ЯЗЫКОВ
// ============================================================================
fn extract_functions(lang: Lang, lines: &[&str]) -> Vec<ParsedFunction> {
    if lang == Lang::Python {
        return extract_python_functions(lines);
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
            r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:const\s+|async\s+|unsafe\s+|extern(?:\s+\x22[^\x22]+\x22)?\s+)*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(([^)]*)\)",
        ).unwrap(),
        _ => return funcs,
    };

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            i += 1;
            continue;
        }

        let mut combined = String::new();
        for offset in 0..6 {
            if i + offset < lines.len() {
                if offset > 0 {
                    combined.push(' ');
                }
                combined.push_str(lines[i + offset].trim());
                if lines[i + offset].contains('{') || lines[i + offset].contains(';') {
                    break;
                }
            }
        }

        if let Some(caps) = re.captures(&combined) {
            let (name, args_str) = extract_name_and_args(&caps);
            if !is_control_keyword(&name) {
                let start_line = i + 1;
                if let Some(end_line) = find_brace_function_end(lines, i) {
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

fn find_brace_function_end(lines: &[&str], start_idx: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut found_open = false;

    for (idx, &line) in lines.iter().enumerate().skip(start_idx) {
        let clean = strip_line_comments_and_strings(line);
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

fn extract_python_functions(lines: &[&str]) -> Vec<ParsedFunction> {
    let mut funcs = Vec::new();
    let def_re =
        Regex::new(r"^(\s*)(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)").unwrap();

    let mut i = 0;
    while i < lines.len() {
        if let Some(caps) = def_re.captures(lines[i]) {
            let indent = caps[1].chars().count();
            let name = caps[2].to_string();
            let start_line = i + 1;

            let mut sig_acc = caps[3].to_string();
            let mut sig_end_idx = i;
            while !sig_acc.contains(')') && sig_end_idx + 1 < lines.len() {
                sig_end_idx += 1;
                sig_acc.push(' ');
                sig_acc.push_str(lines[sig_end_idx].trim());
            }
            let args_str = sig_acc.split(')').next().unwrap_or("").to_string();
            let (arg_count, params_raw) = parse_arguments(Lang::Python, &args_str, start_line);

            let mut end_line = sig_end_idx + 1;
            for (j, &next_line) in lines.iter().enumerate().skip(sig_end_idx + 1) {
                let trimmed = next_line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let next_indent = next_line.chars().take_while(|c| c.is_whitespace()).count();
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

fn parse_arguments(lang: Lang, raw: &str, line_no: usize) -> (usize, Vec<(String, usize)>) {
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

// ============================================================================
// 4, 5, 7. ФУНКЦИИ (Ф1-Ф7), АРГУМЕНТЫ (А1-А7), СЛОЖНОСТЬ (Ц1-Ц7)
// ============================================================================
fn check_functions_and_args(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    lines: &[&str],
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

        let body_slice = &lines[func.start_line - 1..func.end_line.min(lines.len())];
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

fn calculate_cyclomatic_complexity(lang: Lang, body_lines: &[&str]) -> usize {
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
    for &line in body_lines {
        let clean = strip_line_comments_and_strings(line);
        complexity += kw_re.find_iter(&clean).count();
    }
    complexity
}

// ============================================================================
// 6. КОММЕНТАРИИ ВМЕСТО ДОКУМЕНТАЦИИ (К1 - К7)
// ============================================================================
fn check_comments_vs_docs(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    lines: &[&str],
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
                let prev = lines[def_idx - 1].trim();
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
            if def_idx + 1 < lines.len() {
                let first_body = lines[def_idx + 1].trim();
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
            let prev = lines[check_idx - 1].trim();
            if prev.starts_with('@') || prev.starts_with("#[") || prev.starts_with('[') {
                check_idx -= 1;
                continue;
            }
            break;
        }

        if check_idx == 0 {
            continue;
        }

        let prev_line = lines[check_idx - 1].trim();
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
                        && lines[top_comment_idx - 1].trim().starts_with("//")
                    {
                        top_comment_idx -= 1;
                    }
                    let first_comment = lines[top_comment_idx]
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

// ============================================================================
// 8. МАГИЧЕСКИЕ ЧИСЛА В СРАВНЕНИЯХ (М1 - М7)
// ============================================================================
fn check_magic_numbers(
    path: &str,
    lang: Lang,
    lines: &[&str],
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

    let cmp_right_re =
        Regex::new(r"(?:===|!==|==|!=|<=|>=|<|>)\s*(-?\d+(?:\.\d+)?)\b").unwrap();
    let cmp_left_re =
        Regex::new(r"\b(-?\d+(?:\.\d+)?)\s*(?:===|!==|==|!=|<=|>=|<|>)").unwrap();
    let generic_re = Regex::new(r"<[A-Za-z0-9_,\s]+>").unwrap();

    for (idx, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("private const ")
            || trimmed.starts_with("public const ")
            || trimmed.contains("static final ")
            || trimmed.starts_with("const val ")
        {
            continue;
        }

        let clean = strip_line_comments_and_strings(line)
            .replace("->", "  ")
            .replace("=>", "  ")
            .replace("<<", "  ")
            .replace(">>", "  ");
        let clean = generic_re.replace_all(&clean, " ");

        let mut found_magic = None;

        for caps in cmp_right_re.captures_iter(&clean) {
            let num = &caps[1];
            if !is_allowed_number(num) {
                found_magic = Some(num.to_string());
                break;
            }
        }

        if found_magic.is_none() {
            for caps in cmp_left_re.captures_iter(&clean) {
                let num = &caps[1];
                if !is_allowed_number(num) {
                    found_magic = Some(num.to_string());
                    break;
                }
            }
        }

        if let Some(magic) = found_magic {
            violations.push(make_violation(
                rule_id,
                path,
                Some(idx + 1),
                None,
                format!("Магическое число `{}` в сравнении: {}", magic, trimmed),
            ));
        }
    }
}

fn is_allowed_number(num: &str) -> bool {
    matches!(num, "0" | "1" | "-1" | "0.0" | "1.0" | "-1.0")
}

// ============================================================================
// 9. ИМЕНА ИДЕНТИФИКАТОРОВ (И1 - И7)
// ============================================================================
fn check_naming_conventions(
    path: &str,
    lang: Lang,
    funcs: &[ParsedFunction],
    lines: &[&str],
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

    for (idx, &line) in lines.iter().enumerate() {
        let line_no = idx + 1;
        let clean = strip_line_comments_and_strings(line);

        if let Some(caps) = type_re.captures(&clean) {
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
                if let Some(caps) = py_var_re.captures(&clean) {
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
                if let Some(caps) = js_var_re.captures(&clean) {
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
                if let Some(caps) = go_type_re.captures(&clean) {
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
                if let Some(caps) = go_var_re.captures(&clean) {
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
                if let Some(caps) = kt_var_re.captures(&clean) {
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
                if let Some(caps) = rust_let_re.captures(&clean) {
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
                if let Some(caps) = rust_const_re.captures(&clean) {
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

// ============================================================================
// ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ============================================================================
fn is_snake_case(s: &str) -> bool {
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !trimmed.contains("__")
}

fn is_upper_snake_case(s: &str) -> bool {
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_camel_case(s: &str) -> bool {
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

fn is_pascal_case(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return true;
    };
    first.is_ascii_uppercase() && !s.contains('_') && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_go_valid_name(s: &str, is_test_file: bool) -> bool {
    if is_test_file
        && (s.starts_with("Test_") || s.starts_with("Benchmark_") || s.starts_with("Example_"))
    {
        return true;
    }
    !s.contains('_')
}

fn strip_line_comments_and_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_double = false;
    let mut in_single = false;

    while i < chars.len() {
        let c = chars[i];
        if !in_double && !in_single {
            if c == '#' {
                break;
            }
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                break;
            }
        }
        if c == '\\' && (in_double || in_single) {
            i += 2;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            out.push(' ');
            i += 1;
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
            out.push(' ');
            i += 1;
            continue;
        }
        if in_double || in_single {
            out.push(' ');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut chars = s.trim().chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", collected)
    } else {
        collected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subfolders_pr1_pr2_structure() {
        let input = RepoInput {
            repo_name: "student/pract-repo".to_string(),
            files: vec![
                FileEntry {
                    path: ".gitignore".to_string(),
                    content: Some("__pycache__/\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/README.md".to_string(),
                    content: Some("# PR 1\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/__init__.py".to_string(),
                    content: Some(String::new()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/run.sh".to_string(),
                    content: Some("#!/bin/sh\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/run.bat".to_string(),
                    content: Some("@echo off\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/src/vfs.py".to_string(),
                    content: Some("def parse_vfs(path: str) -> None:\n    \"\"\"Parse VFS.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_1/tests/test_vfs.py".to_string(),
                    content: Some("def test_ok() -> None:\n    \"\"\"Test VFS.\"\"\"\n    assert 1 == 1\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_2/README.md".to_string(),
                    content: Some("# PR 2\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_2/run.sh".to_string(),
                    content: Some("#!/bin/sh\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_2/src/cli.py".to_string(),
                    content: Some("def run_cli() -> None:\n    \"\"\"Run CLI.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_2/tests/test_cli.py".to_string(),
                    content: Some("def test_cli() -> None:\n    \"\"\"Test CLI.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
            ],
            commits: vec![
                CommitEntry {
                    sha: "abcdef1".to_string(),
                    message: "fix(pr1): shorten docstring in vfs to satisfy line length limit"
                        .to_string(),
                },
                CommitEntry {
                    sha: "abcdef2".to_string(),
                    message: "refactor(cli): split parse_args to fix function length limit"
                        .to_string(),
                },
                CommitEntry {
                    sha: "abcdef3".to_string(),
                    message: "refactor: move PR_1 files to related folder".to_string(),
                },
            ],
        };

        let report = run_all_checks(&input);
        assert_eq!(report.detected_projects, vec!["PR_1", "PR_2"]);
        assert_eq!(
            report.total_violations, 0,
            "Не должно быть ошибок для корректных папок PR_1 и PR_2: {:?}",
            report.violations
        );
    }
}
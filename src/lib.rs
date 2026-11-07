pub mod analyzer;
pub mod models;
pub mod parser;
pub mod rules;
pub mod sanitizer;

use analyzer::{
    check_comments_vs_docs, check_functions_and_args, check_generated_files, check_magic_numbers,
    check_naming_conventions, check_review_rules, check_structure,
};
use models::{AnalysisReport, Lang, RepoInput, RuleStatus, Severity};
use parser::extract_functions;
use rules::ALL_RULES;
use sanitizer::sanitize_lines;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

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

        let raw_lines: Vec<&str> = content.lines().collect();
        let lang = Lang::from_path(&file.path);

        check_review_rules(&file.path, &raw_lines, lang, &mut violations);

        if let Some(l) = lang {
            if l != Lang::OtherCode {
                let clean_lines = sanitize_lines(l, content);
                let funcs = extract_functions(l, &raw_lines, &clean_lines);
                check_functions_and_args(&file.path, l, &funcs, &clean_lines, &mut violations);
                check_comments_vs_docs(&file.path, l, &funcs, &raw_lines, &mut violations);
                check_magic_numbers(&file.path, l, &raw_lines, &clean_lines, &mut violations);
                check_naming_conventions(&file.path, l, &funcs, &clean_lines, &mut violations);
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
    let mut warning_rules = 0;

    for r in ALL_RULES {
        let count = *counts.get(r.id).unwrap_or(&0);
        if count == 0 {
            passed_rules += 1;
        } else if r.severity == Severity::Warning {
            warning_rules += 1;
        } else {
            failed_rules += 1;
        }
        rules_summary.push(RuleStatus {
            rule_id: r.id.to_string(),
            category: r.category.to_string(),
            description: r.desc.to_string(),
            severity: r.severity,
            count,
            passed: count == 0,
        });
    }

    let critical_violations = violations
        .iter()
        .filter(|v| v.severity == Severity::Error)
        .count();
    let warning_violations = violations
        .iter()
        .filter(|v| v.severity == Severity::Warning)
        .count();

    let project_labels = detected_projects
        .into_iter()
        .map(|p| {
            if p.is_empty() {
                "/ (корень)".to_string()
            } else {
                p
            }
        })
        .collect();

    AnalysisReport {
        repo_name: input.repo_name.clone(),
        detected_projects: project_labels,
        total_files: input.files.len(),
        total_commits: input.commits.len(),
        total_rules: ALL_RULES.len(),
        passed_rules,
        failed_rules,
        warning_rules,
        total_violations: violations.len(),
        critical_violations,
        warning_violations,
        rules_summary,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitEntry, FileEntry};

    #[test]
    fn test_magic_numbers_ignore_version_strings_and_multiline_literals() {
        let py_code = r#"
def check_apk_packages() -> None:
    """Docstring."""
    apk_index = """
    musl>=1.2.0 so:libz.so.1 so:libmissing.so.9 !curl-dev
    """
    pkg_line = "musl>=1.2.0 so:libz.so.1"
    if len(pkg_line) > 1:
        pass
"#;
        let input = RepoInput {
            repo_name: "Aragornyam/test".to_string(),
            files: vec![
                FileEntry {
                    path: ".gitignore".to_string(),
                    content: Some(String::new()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR2/README.md".to_string(),
                    content: Some("# PR2".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR2/run.sh".to_string(),
                    content: Some("#!/bin/sh".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR2/src/app.py".to_string(),
                    content: Some(py_code.to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR2/tests/support.py".to_string(),
                    content: Some(py_code.to_string()),
                    is_binary: false,
                },
            ],
            commits: vec![CommitEntry {
                sha: "11223344".to_string(),
                message: "feat(pr2): add apk parser".to_string(),
            }],
        };

        let report = run_all_checks(&input);
        assert_eq!(report.total_rules, 59);
        assert_eq!(report.passed_rules, 59);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_arbitrary_folders_and_non_critical_src_tests() {
        let input = RepoInput {
            repo_name: "Aragornyam/practices".to_string(),
            files: vec![
                FileEntry {
                    path: ".gitignore".to_string(),
                    content: Some(String::new()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_3/README.md".to_string(),
                    content: Some("# PR 3".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_3/run.sh".to_string(),
                    content: Some("#!/bin/sh".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "PR_3/main.py".to_string(),
                    content: Some("def main() -> None:\n    \"\"\"Run.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "custom_task_name/README.md".to_string(),
                    content: Some("# Custom".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "custom_task_name/Makefile".to_string(),
                    content: Some("all:\n\techo ok".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "custom_task_name/src/app.py".to_string(),
                    content: Some("def run() -> None:\n    \"\"\"Run.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
                FileEntry {
                    path: "custom_task_name/tests/test_app.py".to_string(),
                    content: Some("def test_run() -> None:\n    \"\"\"Test.\"\"\"\n    pass\n".to_string()),
                    is_binary: false,
                },
            ],
            commits: vec![],
        };

        let report = run_all_checks(&input);
        assert_eq!(
            report.detected_projects,
            vec!["PR_3", "custom_task_name"]
        );
        assert_eq!(report.critical_violations, 0);
        assert_eq!(report.warning_violations, 2);
        assert!(report
            .violations
            .iter()
            .all(|v| v.severity == Severity::Warning && (v.rule_id == "С3" || v.rule_id == "С4")));
    }
}
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Serialize, Debug, Clone)]
pub struct Violation {
    pub rule_id: String,
    pub category: String,
    pub description: String,
    pub severity: Severity,
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
    pub severity: Severity,
    pub count: usize,
    pub passed: bool,
}

#[derive(Serialize, Debug)]
pub struct AnalysisReport {
    pub repo_name: String,
    pub detected_projects: Vec<String>,
    pub total_files: usize,
    pub total_commits: usize,
    pub total_rules: usize,
    pub passed_rules: usize,
    pub failed_rules: usize,
    pub warning_rules: usize,
    pub total_violations: usize,
    pub critical_violations: usize,
    pub warning_violations: usize,
    pub rules_summary: Vec<RuleStatus>,
    pub violations: Vec<Violation>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
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
    pub fn from_path(path: &str) -> Option<Self> {
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
pub struct ParsedFunction {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub arg_count: usize,
    pub params_raw: Vec<(String, usize)>,
}
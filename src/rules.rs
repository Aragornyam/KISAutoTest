use crate::models::{Severity, Violation};

pub struct RuleCatalogItem {
    pub id: &'static str,
    pub category: &'static str,
    pub desc: &'static str,
    pub severity: Severity,
}

pub const ALL_RULES: &[RuleCatalogItem] = &[
    RuleCatalogItem { id: "С1", category: "Структура репозитория", desc: "В репозитории отсутствует файл README.md", severity: Severity::Error },
    RuleCatalogItem { id: "С2", category: "Структура репозитория", desc: "В репозитории отсутствует файл .gitignore", severity: Severity::Error },
    RuleCatalogItem { id: "С3", category: "Структура репозитория", desc: "В репозитории отсутствует папка с исходным кодом src", severity: Severity::Warning },
    RuleCatalogItem { id: "С4", category: "Структура репозитория", desc: "В репозитории отсутствует папка с тестами tests", severity: Severity::Warning },
    RuleCatalogItem { id: "С5", category: "Структура репозитория", desc: "В репозитории отсутствует скрипт запуска run.sh, run.bat или Makefile", severity: Severity::Error },
    RuleCatalogItem { id: "С6", category: "Структура репозитория", desc: "Коммиты не соответствуют спецификации Conventional/Scoped Commits", severity: Severity::Error },

    RuleCatalogItem { id: "Г1", category: "Сгенерированные файлы", desc: "Репозиторий содержит архивы", severity: Severity::Error },
    RuleCatalogItem { id: "Г2", category: "Сгенерированные файлы", desc: "Репозиторий содержит бинарные файлы", severity: Severity::Error },
    RuleCatalogItem { id: "Г3", category: "Сгенерированные файлы", desc: "Репозиторий содержит служебные файлы языка Python", severity: Severity::Error },
    RuleCatalogItem { id: "Г4", category: "Сгенерированные файлы", desc: "Репозиторий содержит артефакты сборки", severity: Severity::Error },
    RuleCatalogItem { id: "Г5", category: "Сгенерированные файлы", desc: "Репозиторий содержит файлы с настройками редактора кода", severity: Severity::Error },

    RuleCatalogItem { id: "Р1", category: "Рецензирование", desc: "Репозиторий содержит слишком большие файлы (>1000 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Р2", category: "Рецензирование", desc: "Код на языке Python содержит слишком длинные строки (>80 символов)", severity: Severity::Error },
    RuleCatalogItem { id: "Р3", category: "Рецензирование", desc: "Код на языке JavaScript содержит слишком длинные строки (>80 символов)", severity: Severity::Error },
    RuleCatalogItem { id: "Р4", category: "Рецензирование", desc: "Код на языке Kotlin содержит слишком длинные строки (>100 символов)", severity: Severity::Error },
    RuleCatalogItem { id: "Р5", category: "Рецензирование", desc: "Код на языке Rust содержит слишком длинные строки (>100 символов)", severity: Severity::Error },
    RuleCatalogItem { id: "Р6", category: "Рецензирование", desc: "Код в репозитории содержит слишком длинные строки (>120 символов)", severity: Severity::Error },

    RuleCatalogItem { id: "Ф1", category: "Функции", desc: "Код на языке Python содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф2", category: "Функции", desc: "Код на языке JavaScript содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф3", category: "Функции", desc: "Код на языке C# содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф4", category: "Функции", desc: "Код на языке Java содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф5", category: "Функции", desc: "Код на языке Go содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф6", category: "Функции", desc: "Код на языке Kotlin содержит слишком длинные функции (>40 строк)", severity: Severity::Error },
    RuleCatalogItem { id: "Ф7", category: "Функции", desc: "Код на языке Rust содержит слишком длинные функции (>40 строк)", severity: Severity::Error },

    RuleCatalogItem { id: "А1", category: "Аргументы функций", desc: "Код на Python содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А2", category: "Аргументы функций", desc: "Код на JavaScript содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А3", category: "Аргументы функций", desc: "Код на C# содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А4", category: "Аргументы функций", desc: "Код на Java содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А5", category: "Аргументы функций", desc: "Код на Go содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А6", category: "Аргументы функций", desc: "Код на Kotlin содержит функции с большим числом аргументов (>7)", severity: Severity::Error },
    RuleCatalogItem { id: "А7", category: "Аргументы функций", desc: "Код на Rust содержит функции с большим числом аргументов (>7)", severity: Severity::Error },

    RuleCatalogItem { id: "К1", category: "Комментарии вместо документации", desc: "Код на Python содержит комментарии вместо документации docstring", severity: Severity::Error },
    RuleCatalogItem { id: "К2", category: "Комментарии вместо документации", desc: "Код на JavaScript содержит комментарии вместо документации JSDoc", severity: Severity::Error },
    RuleCatalogItem { id: "К3", category: "Комментарии вместо документации", desc: "Код на C# содержит комментарии вместо документации XMLDoc", severity: Severity::Error },
    RuleCatalogItem { id: "К4", category: "Комментарии вместо документации", desc: "Код на Java содержит комментарии вместо документации Javadoc", severity: Severity::Error },
    RuleCatalogItem { id: "К5", category: "Комментарии вместо документации", desc: "Код на Go содержит комментарии вместо документации Godoc", severity: Severity::Error },
    RuleCatalogItem { id: "К6", category: "Комментарии вместо документации", desc: "Код на Kotlin содержит комментарии вместо документации KDoc", severity: Severity::Error },
    RuleCatalogItem { id: "К7", category: "Комментарии вместо документации", desc: "Код на Rust содержит комментарии вместо документации rustdoc", severity: Severity::Error },

    RuleCatalogItem { id: "Ц1", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Python превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц2", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке JavaScript превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц3", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке C# превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц4", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Java превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц5", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Go превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц6", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Kotlin превышена (>10)", severity: Severity::Error },
    RuleCatalogItem { id: "Ц7", category: "Цикломатическая сложность кода", desc: "Цикломатическая сложность кода на языке Rust превышена (>10)", severity: Severity::Error },

    RuleCatalogItem { id: "М1", category: "Магические числа", desc: "Код на Python содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М2", category: "Магические числа", desc: "Код на JavaScript содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М3", category: "Магические числа", desc: "Код на C# содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М4", category: "Магические числа", desc: "Код на Java содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М5", category: "Магические числа", desc: "Код на Go содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М6", category: "Магические числа", desc: "Код на Kotlin содержит магические числовые литералы в сравнениях", severity: Severity::Error },
    RuleCatalogItem { id: "М7", category: "Магические числа", desc: "Код на Rust содержит магические числовые литералы в сравнениях", severity: Severity::Error },

    RuleCatalogItem { id: "И1", category: "Имена идентификаторов", desc: "Код на Python содержит имена, не соответствующие стандарту PEP8", severity: Severity::Error },
    RuleCatalogItem { id: "И2", category: "Имена идентификаторов", desc: "Код на JavaScript содержит имена, нарушающие соглашения ESLint", severity: Severity::Error },
    RuleCatalogItem { id: "И3", category: "Имена идентификаторов", desc: "Код на C# содержит имена, нарушающие соглашения .NET Runtime", severity: Severity::Error },
    RuleCatalogItem { id: "И4", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Java", severity: Severity::Error },
    RuleCatalogItem { id: "И5", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Go", severity: Severity::Error },
    RuleCatalogItem { id: "И6", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Kotlin", severity: Severity::Error },
    RuleCatalogItem { id: "И7", category: "Имена идентификаторов", desc: "Код содержит имена, нарушающие стандартные соглашения Rust", severity: Severity::Error },
];

pub fn make_violation(
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
        severity: item.severity,
        file: file.into(),
        line,
        end_line,
        snippet: snippet.into(),
    }
}
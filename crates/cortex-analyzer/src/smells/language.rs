use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceLanguage {
    Rust,
    Python,
    Ruby,
    JavaScript,
    TypeScript,
    Java,
    CSharp,
    Go,
    Php,
    Kotlin,
    Scala,
    Swift,
    Json,
    Shell,
    ObjectiveC,
    CLike,
    Unknown,
}

impl SourceLanguage {
    pub(crate) fn from_file_path(path: &str) -> Self {
        let ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("rs") => Self::Rust,
            Some("py") => Self::Python,
            Some("rb") => Self::Ruby,
            Some("js") | Some("jsx") => Self::JavaScript,
            Some("ts") | Some("tsx") => Self::TypeScript,
            Some("java") => Self::Java,
            Some("cs") => Self::CSharp,
            Some("go") => Self::Go,
            Some("php") => Self::Php,
            Some("kt") | Some("kts") => Self::Kotlin,
            Some("scala") => Self::Scala,
            Some("swift") => Self::Swift,
            Some("json") => Self::Json,
            Some("sh") | Some("bash") | Some("zsh") => Self::Shell,
            Some("m") | Some("mm") => Self::ObjectiveC,
            Some("c") | Some("cc") | Some("cpp") | Some("h") | Some("hpp") => Self::CLike,
            _ => Self::Unknown,
        }
    }
}

pub(crate) fn is_comment_line(trimmed: &str, lang: SourceLanguage) -> bool {
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("<!--")
    {
        return true;
    }
    (matches!(
        lang,
        SourceLanguage::Python | SourceLanguage::Ruby | SourceLanguage::Php | SourceLanguage::Shell
    )) && trimmed.starts_with('#')
}

pub(crate) fn is_python_style_function(trimmed: &str, lang: SourceLanguage) -> bool {
    matches!(lang, SourceLanguage::Python)
        && (trimmed.starts_with("def ") || trimmed.starts_with("async def "))
}

pub(crate) fn is_ruby_style_function(trimmed: &str, lang: SourceLanguage) -> bool {
    matches!(lang, SourceLanguage::Ruby) && trimmed.starts_with("def ")
}

pub(crate) fn ruby_block_delta(trimmed: &str) -> i32 {
    let first_token = trimmed
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|s| !s.is_empty());

    let mut starts = 0;
    if matches!(
        first_token,
        Some("def")
            | Some("class")
            | Some("module")
            | Some("if")
            | Some("unless")
            | Some("case")
            | Some("while")
            | Some("until")
            | Some("for")
            | Some("begin")
    ) {
        starts += 1;
    }
    if trimmed.contains(" do ") || trimmed.ends_with(" do") || trimmed.contains(" do |") {
        starts += 1;
    }

    let ends = trimmed
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| *w == "end")
        .count() as i32;

    starts - ends
}

pub(crate) fn is_function_signature(trimmed: &str, lang: SourceLanguage) -> bool {
    if trimmed.is_empty()
        || trimmed.starts_with("if ")
        || trimmed.starts_with("else ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("switch ")
        || trimmed.starts_with("match ")
        || trimmed.starts_with("catch ")
        || trimmed.starts_with("class ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
    {
        return false;
    }

    match lang {
        SourceLanguage::Python => trimmed.starts_with("def ") || trimmed.starts_with("async def "),
        SourceLanguage::Ruby => trimmed.starts_with("def "),
        SourceLanguage::Rust => trimmed.starts_with("fn ") || trimmed.contains(" fn "),
        SourceLanguage::Kotlin => trimmed.starts_with("fun ") || trimmed.contains(" fun "),
        SourceLanguage::Scala => trimmed.starts_with("def ") || trimmed.contains(" def "),
        SourceLanguage::JavaScript | SourceLanguage::TypeScript => {
            trimmed.starts_with("function ")
                || trimmed.starts_with("async function ")
                || (trimmed.contains("=>") && trimmed.contains('=') && trimmed.contains('('))
                || (trimmed.contains('(')
                    && trimmed.contains(')')
                    && trimmed.contains('{')
                    && !trimmed.ends_with(';'))
        }
        SourceLanguage::Php => {
            trimmed.starts_with("function ")
                || (trimmed.contains(" function ") && trimmed.contains('('))
        }
        SourceLanguage::Json => false,
        SourceLanguage::Shell => {
            (trimmed.contains("()")
                && (trimmed.ends_with('{')
                    || trimmed.ends_with("{")
                    || trimmed.contains("() {")
                    || trimmed.contains("(){")))
                || (trimmed.starts_with("function ")
                    && (trimmed.ends_with('{') || trimmed.ends_with("{")))
        }
        SourceLanguage::Java
        | SourceLanguage::CSharp
        | SourceLanguage::Go
        | SourceLanguage::Swift
        | SourceLanguage::ObjectiveC
        | SourceLanguage::CLike => {
            (trimmed.contains('(')
                && trimmed.contains(')')
                && (trimmed.ends_with('{') || trimmed.contains(") {") || trimmed.contains("){")))
                && !trimmed.ends_with(';')
        }
        SourceLanguage::Unknown => {
            (trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || trimmed.contains(" fn ")
                || (trimmed.contains('(') && trimmed.contains(')') && trimmed.contains('{')))
                && !trimmed.ends_with(';')
        }
    }
}

pub(crate) fn is_method_signature(trimmed: &str, lang: SourceLanguage) -> bool {
    is_function_signature(trimmed, lang)
}

pub(crate) fn extract_function_name(line: &str) -> String {
    let mut line = line.trim();
    let prefixes = [
        "pub ",
        "pub(crate) ",
        "pub(super) ",
        "private ",
        "protected ",
        "internal ",
        "static ",
        "async ",
        "extern ",
        "unsafe ",
        "virtual ",
        "override ",
        "final ",
        "abstract ",
    ];
    loop {
        let mut stripped = false;
        for prefix in prefixes {
            if line.starts_with(prefix) {
                line = &line[prefix.len()..];
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    for keyword in ["fn ", "def ", "function ", "fun "] {
        if line.starts_with(keyword) {
            line = &line[keyword.len()..];
            break;
        }
    }

    if let Some(eq_pos) = line.find('=') {
        let left = line[..eq_pos].trim();
        if let Some(name) = left
            .split_whitespace()
            .last()
            .filter(|name| !name.is_empty())
        {
            return name
                .trim_matches(|c: char| c == ':' || c == ',')
                .to_string();
        }
    }

    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|s| {
            !s.is_empty()
                && s.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
        })
        .unwrap_or("unknown")
        .to_string()
}

pub(crate) fn is_python_function_end(
    lines: &[&str],
    current_idx: usize,
    function_start: usize,
) -> bool {
    if current_idx <= function_start {
        return false;
    }

    let function_indent = lines[function_start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .count();

    for next_line in lines.iter().skip(current_idx + 1) {
        let trimmed = next_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let next_indent = next_line.chars().take_while(|c| c.is_whitespace()).count();
        return next_indent <= function_indent;
    }
    true
}

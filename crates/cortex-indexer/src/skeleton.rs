//! Skeleton Precomputation for Fast File Previews
//!
//! Precomputes file skeletons at index time for fast retrieval.
//! Supports multiple modes: minimal (signatures only) and standard (with docstrings).
//!
//! # Compression Features
//!
//! - Smart docstring truncation: Keeps first sentence, truncates with `...`
//! - Parameter type summarization: Compresses long generic types
//! - Struct field compression: Keeps field names, shows type hints

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Maximum docstring lines to include before truncation
const MAX_DOCSTRING_LINES: usize = 5;

/// Maximum characters for a single docstring line
const MAX_DOCSTRING_LINE_CHARS: usize = 120;

/// Maximum characters for a type annotation before summarization
const MAX_TYPE_LENGTH: usize = 60;

/// Maximum struct fields to include before truncation
const MAX_STRUCT_FIELDS: usize = 15;

/// Skeleton cache using sled for persistence
pub struct SkeletonCache {
    db: sled::Db,
}

impl SkeletonCache {
    /// Open or create the skeleton cache at the default location
    pub fn open() -> Result<Self, sled::Error> {
        let path = Self::cache_path();
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Open or create the skeleton cache at a custom location
    pub fn open_at(path: impl AsRef<std::path::Path>) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Get the default cache path
    pub fn cache_path() -> PathBuf {
        if let Ok(path) = std::env::var("CORTEX_SKELETON_CACHE_PATH") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/skeletons.db")
    }

    /// Generate the cache key for a file
    fn make_key(file_path: &str, mode: &str) -> Vec<u8> {
        format!("{}:{}", file_path, mode).into_bytes()
    }

    /// Get a precomputed skeleton
    pub fn get(&self, file_path: &str, mode: &str) -> Option<PrecomputedSkeleton> {
        let key = Self::make_key(file_path, mode);
        let bytes = self.db.get(key).ok()??;
        serde_json::from_slice(&bytes).ok()
    }

    /// Store a precomputed skeleton
    pub fn put(&self, file_path: &str, skeleton: &PrecomputedSkeleton) -> Result<(), sled::Error> {
        let key_minimal = Self::make_key(file_path, "minimal");
        let key_standard = Self::make_key(file_path, "standard");

        // Store under both keys for quick lookup by mode
        let bytes = serde_json::to_vec(skeleton).unwrap_or_default();

        if skeleton.minimal == skeleton.standard {
            // Same content, store once
            self.db.insert(key_minimal, bytes.clone())?;
        } else {
            // Different content, store minimal version
            let minimal_skeleton = PrecomputedSkeleton {
                file_hash: skeleton.file_hash.clone(),
                minimal: skeleton.minimal.clone(),
                standard: skeleton.minimal.clone(), // For minimal key, standard = minimal
                compression_ratio_minimal: skeleton.compression_ratio_minimal,
                compression_ratio_standard: skeleton.compression_ratio_minimal,
            };
            self.db.insert(
                key_minimal,
                serde_json::to_vec(&minimal_skeleton).unwrap_or_default(),
            )?;

            // Store standard version
            self.db.insert(key_standard, bytes)?;
        }

        Ok(())
    }

    /// Remove a skeleton from the cache
    pub fn remove(&self, file_path: &str) -> Result<(), sled::Error> {
        self.db.remove(Self::make_key(file_path, "minimal"))?;
        self.db.remove(Self::make_key(file_path, "standard"))?;
        Ok(())
    }

    /// Clear all cached skeletons
    pub fn clear(&self) -> Result<(), sled::Error> {
        self.db.clear()?;
        Ok(())
    }

    /// Get the number of cached skeletons
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }
}

/// A precomputed skeleton with multiple modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecomputedSkeleton {
    /// Hash of the source file
    pub file_hash: String,
    /// Minimal skeleton (signatures only)
    pub minimal: String,
    /// Standard skeleton (with docstrings)
    pub standard: String,
    /// Compression ratio for minimal mode
    pub compression_ratio_minimal: f64,
    /// Compression ratio for standard mode
    pub compression_ratio_standard: f64,
}

impl PrecomputedSkeleton {
    /// Create a new precomputed skeleton from source content
    pub fn new(source: &str, file_hash: String) -> Self {
        let minimal = build_skeleton(source, "minimal");
        let standard = build_skeleton(source, "standard");

        let source_len = source.len().max(1) as f64;
        let compression_ratio_minimal = minimal.len() as f64 / source_len;
        let compression_ratio_standard = standard.len() as f64 / source_len;

        Self {
            file_hash,
            minimal,
            standard,
            compression_ratio_minimal,
            compression_ratio_standard,
        }
    }

    /// Get the skeleton for a specific mode
    pub fn get_skeleton(&self, mode: &str) -> &str {
        match mode {
            "standard" => &self.standard,
            _ => &self.minimal,
        }
    }

    /// Get the compression ratio for a specific mode
    pub fn compression_ratio(&self, mode: &str) -> f64 {
        match mode {
            "standard" => self.compression_ratio_standard,
            _ => self.compression_ratio_minimal,
        }
    }
}

/// Build a skeleton from source code with enhanced compression
///
/// # Modes
/// - "minimal": Signatures only, compressed for maximum brevity
/// - "standard": Includes truncated docstrings and import statements
///
/// # Compression Features
/// - Docstrings truncated to first sentence
/// - Long types summarized (e.g., `HashMap<String, Vec<...>>`)
/// - Struct fields compressed to essential info
pub fn build_skeleton(content: &str, mode: &str) -> String {
    let mut out = Vec::new();
    let mut in_block_comment = false;
    let mut current_docstring: Vec<String> = Vec::new();
    let mut docstring_line_count = 0;

    let lines: Vec<&str> = content.lines().collect();
    let max_line_width = if mode == "minimal" { 100 } else { 120 };

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Handle block comments (/* */)
        if trimmed.starts_with("/*") {
            in_block_comment = true;
            if mode == "standard" {
                current_docstring.push((*line).to_string());
            }
            continue;
        }

        if in_block_comment {
            if mode == "standard" {
                current_docstring.push((*line).to_string());
            }
            if trimmed.ends_with("*/") {
                in_block_comment = false;
            }
            continue;
        }

        // Collect docstrings for standard mode (with line limit)
        if mode == "standard" && (trimmed.starts_with("///") || trimmed.starts_with("//!")) {
            docstring_line_count += 1;
            if docstring_line_count <= MAX_DOCSTRING_LINES {
                // Truncate long docstring lines
                let compressed_doc = if trimmed.len() > MAX_DOCSTRING_LINE_CHARS {
                    let content_part = trimmed.trim_start_matches('/');
                    let truncated = truncate_docstring(content_part, MAX_DOCSTRING_LINE_CHARS - 4);
                    format!("///{}", truncated)
                } else {
                    (*line).to_string()
                };
                current_docstring.push(compressed_doc);
            }
            continue;
        }

        // Python-style docstrings (simple heuristic)
        if mode == "standard" && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
            current_docstring.push((*line).to_string());
            continue;
        }

        // Check for signature patterns
        let is_signature = is_signature_line(trimmed);

        if is_signature {
            // Add collected docstrings before the signature
            if mode == "standard" && !current_docstring.is_empty() {
                for doc_line in &current_docstring {
                    out.push(doc_line.clone());
                }
                // Add truncation indicator if docstring was cut short
                if docstring_line_count > MAX_DOCSTRING_LINES {
                    out.push("/// ...".to_string());
                }
            }
            current_docstring.clear();
            docstring_line_count = 0;

            // Apply line compression for long signatures
            let compressed_line = compress_line(line, max_line_width);
            out.push(compressed_line);

            // For minimal mode, limit output size
            if mode == "minimal" && out.len() >= 120 {
                break;
            }
        } else {
            // Clear docstrings if not followed by a signature
            current_docstring.clear();
            docstring_line_count = 0;
        }

        // For standard mode, include imports at the top
        if mode == "standard"
            && i < 20
            && (trimmed.starts_with("use ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("#include"))
            && !out.contains(&(*line).to_string())
        {
            out.push((*line).to_string());
        }
    }

    // If no signatures found, return a truncated version of the source
    if out.is_empty() {
        return lines
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
    }

    out.join("\n")
}

/// Check if a line represents a code signature
fn is_signature_line(trimmed: &str) -> bool {
    // Rust
    trimmed.starts_with("pub fn ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub struct ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("pub enum ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("pub trait ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("pub impl ")
        // Python
        || trimmed.starts_with("def ")
        || trimmed.starts_with("async def ")
        || trimmed.starts_with("class ")
        // JavaScript/TypeScript
        || trimmed.starts_with("function ")
        || trimmed.starts_with("export function ")
        || trimmed.starts_with("async function ")
        || trimmed.starts_with("export async function ")
        || (trimmed.starts_with("const ") && trimmed.contains("= (")) // Arrow functions
        || trimmed.starts_with("export const ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("type ")
        // Java/C++/C#
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private ")
        || trimmed.starts_with("protected ")
        || (trimmed.starts_with("class ") && trimmed.ends_with('{'))
        // Go
        || trimmed.starts_with("func ")
        || (trimmed.starts_with("type ") && trimmed.contains("struct"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Enhanced Compression Functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Truncate a docstring to keep only the first meaningful sentence
///
/// # Example
/// ```
/// let doc = "This function does something. It also does more.";
/// let truncated = truncate_docstring(doc, 50);
/// assert_eq!(truncated, "This function does something...");
/// ```
pub fn truncate_docstring(doc: &str, max_chars: usize) -> String {
    let doc = doc.trim();

    // If already short enough, return as-is
    if doc.len() <= max_chars {
        return doc.to_string();
    }

    // Try to find first sentence end (., !, ?)
    let sentence_end_chars = ['.', '!', '?'];
    for end_char in sentence_end_chars {
        if let Some(pos) = doc.find(end_char) {
            let first_sentence = &doc[..=pos];
            if first_sentence.len() <= max_chars {
                return format!("{} ...", first_sentence.trim());
            }
        }
    }

    // Try to find a logical break point (comma, semicolon)
    let break_chars = [',', ';', ':'];
    for break_char in break_chars {
        if let Some(pos) = doc.find(break_char) {
            let first_part = &doc[..=pos];
            if first_part.len() <= max_chars {
                return format!("{} ...", first_part.trim());
            }
        }
    }

    // Fall back to word boundary truncation
    let truncated = &doc[..max_chars.saturating_sub(4).max(20)];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{} ...", &truncated[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

/// Summarize a complex type annotation
///
/// Compresses long generic types and nested structures
///
/// # Example
/// ```
/// let ty = "HashMap<String, Vec<Result<MyCustomType, Box<dyn Error>>>>";
/// let summarized = summarize_type(ty, 30);
/// assert_eq!(summarized, "HashMap<String, Vec<...>>");
/// ```
pub fn summarize_type(ty: &str, max_len: usize) -> String {
    let ty = ty.trim();

    // If already short enough, return as-is
    if ty.len() <= max_len {
        return ty.to_string();
    }

    // Count angle brackets to detect generics
    let open_count = ty.chars().filter(|&c| c == '<').count();
    let close_count = ty.chars().filter(|&c| c == '>').count();

    if open_count > 0 && open_count == close_count {
        // It's a generic type - try to summarize
        if let Some(first_angle) = ty.find('<') {
            let base_type = &ty[..first_angle];

            // Find the matching closing bracket
            let mut depth = 0;
            let mut last_close = 0;
            for (i, c) in ty[first_angle..].char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            last_close = first_angle + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            // Check if we can keep the first type parameter
            let inner = &ty[first_angle + 1..last_close];
            let inner_parts: Vec<&str> = inner.split(',').collect();

            if inner_parts.len() > 1 {
                // Multiple type parameters - keep first and abbreviate
                let first_param = inner_parts[0].trim();
                if base_type.len() + first_param.len() + 8 <= max_len {
                    return format!("{}<{}, ...>", base_type, first_param);
                }
            }

            // Single nested type or too long - just show base
            if base_type.len() + 5 <= max_len {
                return format!("{}<...>", base_type);
            }
        }
    }

    // Check for function pointer types
    if (ty.starts_with("fn(") || ty.starts_with("Fn(") || ty.starts_with("impl Fn("))
        && let Some(paren_end) = find_matching_paren(ty)
    {
        let params = &ty[..=paren_end];
        if params.len() + 8 <= max_len {
            return format!("{} -> ...", params);
        }
        // Just show fn(...) -> ...
        return "fn(...) -> ...".to_string();
    }

    // Check for Option/Result types
    for wrapper in ["Option<", "Result<", "Box<", "Arc<", "Rc<", "Vec<"] {
        if ty.starts_with(wrapper) {
            let inner = &ty[wrapper.len()..ty.len().saturating_sub(1)];
            let summarized_inner = summarize_type(inner, max_len.saturating_sub(wrapper.len() + 3));
            return format!("{}{}>", &wrapper[..wrapper.len() - 1], summarized_inner);
        }
    }

    // Fall back to truncation with ellipsis
    if ty.len() > max_len {
        format!("{}...", &ty[..max_len.saturating_sub(3)])
    } else {
        ty.to_string()
    }
}

/// Find the matching closing parenthesis for the first opening one
fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Compress struct fields by keeping names and summarizing types
///
/// # Example
/// ```
/// let fields = r#"    pub name: String,
///     pub age: u32,
///     pub data: HashMap<String, Vec<Result<Data, Error>>>,
///     pub extra: Option<Box<dyn Any>>"#;
/// let compressed = compress_struct_fields(fields, 60);
/// assert!(compressed.contains("name: String"));
/// assert!(compressed.contains("HashMap<...>"));
/// ```
pub fn compress_struct_fields(fields: &str, max_total_width: usize) -> String {
    let lines: Vec<&str> = fields.lines().collect();

    // If few fields and short, return as-is
    if lines.len() <= 5 && fields.len() <= max_total_width {
        return fields.to_string();
    }

    let mut compressed = Vec::new();
    let mut field_count = 0;

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        field_count += 1;

        // Stop after max fields
        if field_count > MAX_STRUCT_FIELDS {
            compressed.push("    // ... more fields".to_string());
            break;
        }

        // Parse field: "pub name: Type" or "name: Type"
        if let Some(colon_pos) = trimmed.find(':') {
            let field_name = trimmed[..colon_pos].trim();
            let type_part = trimmed[colon_pos + 1..].trim_end_matches(',').trim();

            // Summarize the type if needed
            let available_width = max_total_width.saturating_sub(field_name.len() + 4);
            let max_type = available_width.min(MAX_TYPE_LENGTH);
            let summarized_type = summarize_type(type_part, max_type);

            compressed.push(format!("    {}: {}", field_name, summarized_type));
        } else {
            // Keep line as-is if we can't parse it
            compressed.push(format!("    {}", trimmed));
        }
    }

    compressed.join("\n")
}

/// Smart line compression that preserves semantic meaning
pub fn compress_line(line: &str, max_width: usize) -> String {
    let trimmed = line.trim();

    // Skip short lines
    if trimmed.len() <= max_width {
        return line.to_string();
    }

    // Check for function signatures
    if trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("async fn ")
    {
        return compress_function_signature(trimmed, max_width);
    }

    // Check for struct definitions with fields on same line
    if trimmed.starts_with("struct ") && trimmed.contains('{') {
        return compress_struct_single_line(trimmed, max_width);
    }

    // Default: truncate at word boundary
    if trimmed.len() > max_width {
        let truncated = &trimmed[..max_width.saturating_sub(3)];
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", truncated)
        }
    } else {
        line.to_string()
    }
}

/// Compress a function signature while keeping it readable
fn compress_function_signature(sig: &str, max_width: usize) -> String {
    if sig.len() <= max_width {
        return sig.to_string();
    }

    // Find the parameter list
    if let Some(paren_start) = sig.find('(') {
        // Find matching closing paren
        let mut depth = 0;
        let mut paren_end = paren_start;
        for (i, c) in sig[paren_start..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        paren_end = paren_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let fn_name_part = &sig[..=paren_start];
        let params = &sig[paren_start + 1..paren_end];
        let return_part = &sig[paren_end + 1..];

        // Compress parameters if needed
        let available_for_params =
            max_width.saturating_sub(fn_name_part.len() + return_part.len() + 5);

        if params.len() > available_for_params {
            let param_count = params.split(',').count();
            if param_count > 2 {
                let first_param = params.split(',').next().unwrap_or("");
                let compressed = format!(
                    "{}({first_param}, ... /* +{} params */){}",
                    fn_name_part,
                    param_count - 1,
                    return_part
                );
                if compressed.len() <= max_width {
                    return compressed;
                }
            }
        }
    }

    // Fall back to simple truncation
    format!("{}...", &sig[..max_width.saturating_sub(3)])
}

/// Compress a struct definition on a single line
fn compress_struct_single_line(struct_line: &str, max_width: usize) -> String {
    if struct_line.len() <= max_width {
        return struct_line.to_string();
    }

    // Extract struct name and compress fields
    if let Some(brace_pos) = struct_line.find('{') {
        let name_part = &struct_line[..brace_pos + 1];
        let fields_part = &struct_line[brace_pos + 1..];

        // Count fields
        let field_count = fields_part
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();

        if field_count > 0 {
            return format!("{} /* {} fields */ }}", name_part.trim_end(), field_count);
        }
    }

    struct_line.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// File Hash
// ═══════════════════════════════════════════════════════════════════════════════

/// Compute file hash
pub fn file_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Skeleton builder that integrates with the indexer
pub struct SkeletonBuilder {
    cache: Option<SkeletonCache>,
}

impl SkeletonBuilder {
    /// Create a new skeleton builder without caching
    pub fn new() -> Self {
        Self { cache: None }
    }

    /// Create a skeleton builder with caching
    pub fn with_cache() -> Result<Self, sled::Error> {
        let cache = SkeletonCache::open()?;
        Ok(Self { cache: Some(cache) })
    }

    /// Create a skeleton builder with a custom cache
    pub fn with_cache_at(path: impl AsRef<std::path::Path>) -> Result<Self, sled::Error> {
        let cache = SkeletonCache::open_at(path)?;
        Ok(Self { cache: Some(cache) })
    }

    /// Get or compute a skeleton for a file
    pub fn get_or_compute(
        &self,
        file_path: &str,
        content: &str,
        mode: &str,
    ) -> (String, bool, f64) {
        let hash = file_hash(content);

        // Try cache first
        if let Some(ref cache) = self.cache
            && let Some(skeleton) = cache.get(file_path, mode)
            && skeleton.file_hash == hash
        {
            return (
                skeleton.get_skeleton(mode).to_string(),
                true, // precomputed
                skeleton.compression_ratio(mode),
            );
        }

        // Compute on demand
        let skeleton = build_skeleton(content, mode);
        let compression_ratio = skeleton.len() as f64 / content.len().max(1) as f64;

        // Cache the result
        if let Some(ref cache) = self.cache {
            let precomputed = PrecomputedSkeleton::new(content, hash);
            let _ = cache.put(file_path, &precomputed);
        }

        (skeleton, false, compression_ratio)
    }

    /// Invalidate the cache for a file
    pub fn invalidate(&self, file_path: &str) {
        if let Some(ref cache) = self.cache {
            let _ = cache.remove(file_path);
        }
    }
}

impl Default for SkeletonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_skeleton_rust() {
        let src = r#"
/// Doc comment
pub fn do_something(x: i32) -> i32 {
    x + 1
}

struct MyStruct {
    field: String,
}

fn internal() {}
"#;

        let minimal = build_skeleton(src, "minimal");

        assert!(minimal.contains("pub fn do_something"));
        assert!(minimal.contains("struct MyStruct"));
        assert!(minimal.contains("fn internal"));
        assert!(!minimal.contains("Doc comment"));
    }

    #[test]
    fn build_skeleton_rust_standard() {
        let src = r#"
/// This is a doc comment
pub fn documented() {}

fn undocumented() {}
"#;

        let standard = build_skeleton(src, "standard");

        assert!(standard.contains("/// This is a doc comment"));
        assert!(standard.contains("pub fn documented"));
    }

    #[test]
    fn build_skeleton_python() {
        let src = r#"
def hello_world():
    pass

class MyClass:
    def method(self):
        pass

async def async_func():
    pass
"#;

        let skeleton = build_skeleton(src, "minimal");

        assert!(skeleton.contains("def hello_world"));
        assert!(skeleton.contains("class MyClass"));
        assert!(skeleton.contains("async def async_func"));
    }

    #[test]
    fn build_skeleton_javascript() {
        let src = r#"
function regular() {}

async function asyncFunc() {}

const arrow = () => {};

export function exported() {}

class ESClass {}
"#;

        let skeleton = build_skeleton(src, "minimal");

        assert!(skeleton.contains("function regular"));
        assert!(skeleton.contains("async function asyncFunc"));
        assert!(skeleton.contains("class ESClass"));
    }

    #[test]
    fn build_skeleton_empty_returns_truncated() {
        let src = "just regular text\nno signatures\n".repeat(10);
        let skeleton = build_skeleton(&src, "minimal");

        // Should return first 40 lines
        assert!(skeleton.lines().count() <= 40);
    }

    #[test]
    fn precomputed_skeleton() {
        let src = "pub fn test() {}";
        let hash = file_hash(src);

        let skeleton = PrecomputedSkeleton::new(src, hash.clone());

        assert_eq!(skeleton.file_hash, hash);
        assert!(skeleton.minimal.contains("pub fn test"));
        assert!(skeleton.compression_ratio_minimal > 0.0);
    }

    #[test]
    fn file_hash_deterministic() {
        let content = "test content";
        let hash1 = file_hash(content);
        let hash2 = file_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn skeleton_builder_no_cache() {
        let builder = SkeletonBuilder::new();

        let (skeleton, precomputed, ratio) =
            builder.get_or_compute("test.rs", "pub fn test() {}", "minimal");

        assert!(skeleton.contains("pub fn test"));
        assert!(!precomputed);
        assert!(ratio > 0.0);
    }

    #[test]
    fn is_signature_detects_various_languages() {
        assert!(is_signature_line("pub fn test() {}"));
        assert!(is_signature_line("fn test() {}"));
        assert!(is_signature_line("def test():"));
        assert!(is_signature_line("class Test:"));
        assert!(is_signature_line("function test() {}"));
        assert!(is_signature_line("func test() {}"));
        assert!(is_signature_line("interface Test {}"));
        assert!(is_signature_line("impl Test {}"));

        assert!(!is_signature_line("let x = 1;"));
        assert!(!is_signature_line("// comment"));
        assert!(!is_signature_line("return x;"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Compression Function Tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn truncate_docstring_short_text() {
        let doc = "This is short.";
        let result = truncate_docstring(doc, 50);
        assert_eq!(result, "This is short.");
    }

    #[test]
    fn truncate_docstring_at_sentence() {
        let doc = "This is the first sentence. This is the second sentence.";
        let result = truncate_docstring(doc, 50);
        assert_eq!(result, "This is the first sentence. ...");
    }

    #[test]
    fn truncate_docstring_at_comma() {
        let doc = "This function does something, and also handles edge cases, and more.";
        let result = truncate_docstring(doc, 30);
        assert!(result.contains("..."));
        assert!(result.len() <= 33); // max_chars + "..."
    }

    #[test]
    fn truncate_docstring_word_boundary() {
        let doc = "ThisIsOneVeryLongWordWithoutSpacesThatCannotBeBroken";
        let result = truncate_docstring(doc, 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn summarize_type_short() {
        let ty = "String";
        let result = summarize_type(ty, 30);
        assert_eq!(result, "String");
    }

    #[test]
    fn summarize_type_generic_simple() {
        let ty = "Vec<String>";
        let result = summarize_type(ty, 30);
        assert_eq!(result, "Vec<String>");
    }

    #[test]
    fn summarize_type_generic_nested() {
        let ty = "HashMap<String, Vec<Result<Data, Error>>>";
        let result = summarize_type(ty, 25);
        assert!(result.contains("HashMap<"));
        assert!(result.contains("..."));
    }

    #[test]
    fn summarize_type_generic_multiple_params() {
        let ty = "HashMap<String, i32, RandomState>";
        let result = summarize_type(ty, 30);
        assert!(result.contains("String"));
        assert!(result.contains("..."));
    }

    #[test]
    fn summarize_type_option() {
        let ty = "Option<Box<dyn Any + Send + Sync>>";
        let result = summarize_type(ty, 20);
        assert!(result.contains("Option<"));
    }

    #[test]
    fn compress_line_short() {
        let line = "pub fn test() {}";
        let result = compress_line(line, 80);
        assert_eq!(result, line);
    }

    #[test]
    fn compress_line_function_long_params() {
        let line = "pub fn very_long_function_with_many_parameters(param1: String, param2: i32, param3: bool, param4: Vec<String>) -> Result<(), Error>";
        let result = compress_line(line, 60);
        assert!(result.len() <= 63);
        assert!(result.contains("very_long_function"));
    }

    #[test]
    fn compress_struct_fields_short() {
        let fields = "    name: String,\n    age: u32";
        let result = compress_struct_fields(fields, 80);
        assert!(result.contains("name:"));
        assert!(result.contains("age:"));
    }

    #[test]
    fn compress_struct_fields_with_long_type() {
        let fields = r#"    simple: String,
    complex: HashMap<String, Vec<Result<Data, Box<dyn Error>>>>,
    another: i32"#;
        let result = compress_struct_fields(fields, 40);
        assert!(result.contains("simple:"));
        assert!(result.contains("complex:"));
        // Complex type should be summarized
        assert!(result.contains("HashMap<"));
    }

    #[test]
    fn build_skeleton_compresses_long_docstrings() {
        let src = r#"
/// This is a very long docstring that exceeds the maximum line length and should be truncated
/// to keep only the essential information while preserving readability for the AI model.
pub fn documented_function() {}
"#;
        let standard = build_skeleton(src, "standard");

        // Should contain the function signature
        assert!(standard.contains("pub fn documented_function"));

        // Docstring should be present (possibly truncated)
        assert!(standard.contains("///"));
    }

    #[test]
    fn build_skeleton_limits_docstring_lines() {
        let src = r#"
/// Line 1
/// Line 2
/// Line 3
/// Line 4
/// Line 5
/// Line 6
/// Line 7
/// Line 8
pub fn many_doc_lines() {}
"#;
        let standard = build_skeleton(src, "standard");

        // Should have truncation indicator since there are more than MAX_DOCSTRING_LINES
        assert!(standard.contains("/// ..."));
    }
}

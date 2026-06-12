//! Token counting helpers for MCP response budgeting and savings measurement.

use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

static CL100K: OnceLock<Option<CoreBPE>> = OnceLock::new();

fn cl100k_base() -> Option<&'static CoreBPE> {
    CL100K
        .get_or_init(|| tiktoken_rs::cl100k_base().ok())
        .as_ref()
}

/// Active tokenizer label for envelope metadata.
pub fn tokenizer_name(exact: bool) -> &'static str {
    if exact { "cl100k_base" } else { "chars/4" }
}

/// Count tokens in `text`. Returns `(count, exact)` where `exact` is false when
/// falling back to a chars/4 heuristic because the tokenizer failed to load.
pub fn count_tokens(text: &str) -> (usize, bool) {
    if text.is_empty() {
        return (0, cl100k_base().is_some());
    }
    if let Some(bpe) = cl100k_base() {
        (bpe.encode_with_special_tokens(text).len(), true)
    } else {
        (chars_div4(text.chars().count()), false)
    }
}

/// Estimate how many tokens a full source payload would consume by extrapolating
/// from a bounded sample. Returns `(baseline_tokens, baseline_estimated)`.
pub fn estimate_baseline_from_sample(total_file_chars: usize, sample_text: &str) -> (usize, bool) {
    if total_file_chars == 0 {
        return (0, false);
    }
    if sample_text.is_empty() {
        let (count, exact) = count_tokens_by_chars(total_file_chars);
        return (count, !exact);
    }
    let sample_chars = sample_text.chars().count();
    if sample_chars == 0 {
        let (count, exact) = count_tokens_by_chars(total_file_chars);
        return (count, !exact);
    }
    if sample_chars >= total_file_chars {
        let (count, exact) = count_tokens(sample_text);
        return (count, !exact);
    }
    let (sample_tokens, exact) = count_tokens(sample_text);
    if !exact {
        let baseline = chars_div4(total_file_chars);
        return (baseline, true);
    }
    let ratio = sample_tokens as f64 / sample_chars as f64;
    let baseline = (ratio * total_file_chars as f64).ceil() as usize;
    (baseline.max(sample_tokens), true)
}

fn count_tokens_by_chars(char_count: usize) -> (usize, bool) {
    if char_count == 0 {
        return (0, false);
    }
    (chars_div4(char_count), false)
}

fn chars_div4(char_count: usize) -> usize {
    char_count.div_ceil(4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_tokens_empty_is_zero() {
        let (count, _) = count_tokens("");
        assert_eq!(count, 0);
    }

    #[test]
    fn count_tokens_non_empty() {
        let (count, _) = count_tokens("hello world");
        assert!(count >= 1);
    }

    #[test]
    fn estimate_baseline_from_full_sample() {
        let text = "fn main() { println!(\"hi\"); }";
        let (baseline, estimated) = estimate_baseline_from_sample(text.chars().count(), text);
        let (direct, _) = count_tokens(text);
        assert_eq!(baseline, direct);
        assert!(!estimated);
    }

    #[test]
    fn estimate_baseline_extrapolates_larger_total() {
        let sample = "abcd".repeat(100);
        let (baseline, estimated) =
            estimate_baseline_from_sample(sample.chars().count() * 4, &sample);
        assert!(estimated);
        assert!(baseline >= count_tokens(&sample).0);
    }

    #[test]
    fn tokenizer_name_reflects_exactness() {
        assert_eq!(tokenizer_name(true), "cl100k_base");
        assert_eq!(tokenizer_name(false), "chars/4");
    }
}

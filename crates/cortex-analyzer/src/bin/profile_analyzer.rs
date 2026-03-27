//! Local profiling harness for the same static smell path the CLI uses **without** graph context.
//!
//! Run from the workspace root:
//!
//! ```text
//! cargo run -p cortex-analyzer --release --bin profile_analyzer -- crates/cortex-analyzer/src
//! ```
//!
//! Environment:
//!
//! - `CORTEX_PROFILE_MAX_FILES` — cap files scanned (default `500`)
//! - `CORTEX_PROFILE_MAX_BYTES` — skip larger files (default `1048576`, same order as CLI analyze)
//! - `CORTEX_PROFILE_MAX_LINES` — after reading each file, keep at most this many lines for **all**
//!   timed phases (default `400`). Set to `0` for no limit (can be very slow: `detect_duplicate_code`
//!   and `DuplicationDetector` are superlinear on large inputs).
//! - `CORTEX_PROFILE_CROSS_DUP=1` — also time `find_duplicates` across a **bounded** slice of the
//!   corpus (first `CORTEX_PROFILE_CROSS_DUP_FILES` files, each further truncated to
//!   `CORTEX_PROFILE_CROSS_DUP_MAX_LINES` lines). Cross-file `find_duplicates` is expensive; this is a
//!   sampled stress check.
//!
//! Interpretation: compare per-phase totals. Only add caching when a phase dominates wall time on
//! your real corpus and a profiler confirms hot allocations/branches.

use cortex_analyzer::{
    DuplicationDetector, SmellConfig, SmellDetector, detect_dead_code, detect_duplicate_code,
    detect_feature_envy, detect_inappropriate_intimacy, detect_shotgun_surgery,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "rb", "c", "cc", "cpp", "h", "hpp", "cs",
    "php", "swift", "kt", "kts", "m", "mm", "scala",
];

const SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    ".hg",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
];

#[derive(Default)]
struct PhaseTimings {
    read_fs: Duration,
    smell_detector_detect: Duration,
    detect_dead_code: Duration,
    detect_shotgun_surgery: Duration,
    detect_feature_envy: Duration,
    detect_inappropriate_intimacy: Duration,
    detect_duplicate_code: Duration,
    duplication_find_duplicates_in_file: Duration,
    duplication_find_duplicates_cross_file: Duration,
}

impl PhaseTimings {
    fn print_report(&self, files: usize, bytes: u64, max_lines: usize) {
        println!("=== cortex-analyzer profile ===");
        println!("files: {files}  total_bytes_read: {bytes}");
        if max_lines == 0 {
            println!("CORTEX_PROFILE_MAX_LINES: 0 (no truncation)");
        } else {
            println!("CORTEX_PROFILE_MAX_LINES: {max_lines} (applied after read, all phases)");
        }
        let phases: [(&str, Duration); 9] = [
            ("read_fs + load sources", self.read_fs),
            (
                "SmellDetector::detect (legacy core)",
                self.smell_detector_detect,
            ),
            ("detect_dead_code", self.detect_dead_code),
            ("detect_shotgun_surgery", self.detect_shotgun_surgery),
            ("detect_feature_envy", self.detect_feature_envy),
            (
                "detect_inappropriate_intimacy",
                self.detect_inappropriate_intimacy,
            ),
            (
                "detect_duplicate_code (per-file)",
                self.detect_duplicate_code,
            ),
            (
                "DuplicationDetector::find_duplicates_in_file (per file)",
                self.duplication_find_duplicates_in_file,
            ),
            (
                "DuplicationDetector::find_duplicates (cross-file sample; CORTEX_PROFILE_CROSS_DUP=1)",
                self.duplication_find_duplicates_cross_file,
            ),
        ];
        let total: Duration = phases.iter().map(|(_, d)| *d).sum();
        println!("sum_phases: {:?}", total);
        for (name, d) in phases {
            let pct = if total.is_zero() {
                0.0
            } else {
                d.as_secs_f64() / total.as_secs_f64() * 100.0
            };
            println!("  {pct:5.1}%  {name}: {:?}", d);
        }
    }
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn maybe_truncate_lines(s: &str, max_lines: usize) -> String {
    if max_lines == 0 {
        return s.to_string();
    }
    s.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

fn is_analyzable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            DEFAULT_EXTENSIONS
                .iter()
                .any(|&c| ext.eq_ignore_ascii_case(c))
        })
        .unwrap_or(false)
}

fn collect_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();
    while let Some(p) = stack.pop() {
        if out.len() >= max_files {
            break;
        }
        let Ok(meta) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_file() {
            if is_analyzable(&p) {
                out.push(p);
            }
            continue;
        }
        if !meta.is_dir() {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&p) else {
            continue;
        };
        for ent in rd.flatten() {
            let path = ent.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if SKIP_DIR_NAMES.contains(&name) {
                    continue;
                }
            }
            stack.push(path);
        }
    }
    out.sort();
    out
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    let max_files = env_usize("CORTEX_PROFILE_MAX_FILES", 500);
    let max_bytes = env_u64("CORTEX_PROFILE_MAX_BYTES", 1_048_576);
    let max_lines = env_usize("CORTEX_PROFILE_MAX_LINES", 400);

    if !root.exists() {
        eprintln!("path does not exist: {}", root.display());
        std::process::exit(1);
    }

    let t0 = Instant::now();
    let paths = collect_files(&root, max_files);
    let mut sources: Vec<(String, String)> = Vec::with_capacity(paths.len());
    let mut total_bytes: u64 = 0;

    for path in paths {
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.len() > max_bytes {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = maybe_truncate_lines(&text, max_lines);
        total_bytes += text.len() as u64;
        let fp = path.display().to_string();
        sources.push((fp, text));
    }
    let read_fs = t0.elapsed();

    let detector = SmellDetector::new();
    let smell_config = SmellConfig::default();

    let mut timings = PhaseTimings {
        read_fs,
        ..Default::default()
    };

    for (fp, src) in &sources {
        let s = Instant::now();
        let _ = detector.detect(src, fp);
        timings.smell_detector_detect += s.elapsed();

        let s = Instant::now();
        let _ = detect_dead_code(src, fp, &smell_config);
        timings.detect_dead_code += s.elapsed();

        let s = Instant::now();
        let _ = detect_shotgun_surgery(src, fp, &smell_config);
        timings.detect_shotgun_surgery += s.elapsed();

        let s = Instant::now();
        let _ = detect_feature_envy(src, fp, &smell_config);
        timings.detect_feature_envy += s.elapsed();

        let s = Instant::now();
        let _ = detect_inappropriate_intimacy(src, fp, &smell_config);
        timings.detect_inappropriate_intimacy += s.elapsed();

        let s = Instant::now();
        let _ = detect_duplicate_code(src, fp, &smell_config);
        timings.detect_duplicate_code += s.elapsed();
    }

    let dup_detector = DuplicationDetector::new();
    let s = Instant::now();
    for (fp, src) in &sources {
        let _ = dup_detector.find_duplicates_in_file(src, fp);
    }
    timings.duplication_find_duplicates_in_file = s.elapsed();

    if std::env::var("CORTEX_PROFILE_CROSS_DUP").ok().as_deref() == Some("1") {
        let max_files = env_usize("CORTEX_PROFILE_CROSS_DUP_FILES", 8);
        let max_lines = env_usize("CORTEX_PROFILE_CROSS_DUP_MAX_LINES", 120);
        let sample: Vec<(String, String)> = sources
            .iter()
            .take(max_files)
            .map(|(p, s)| {
                let truncated = maybe_truncate_lines(s, max_lines);
                (p.clone(), truncated)
            })
            .collect();
        let s = Instant::now();
        let _ = dup_detector.find_duplicates(&sample);
        timings.duplication_find_duplicates_cross_file = s.elapsed();
    }

    timings.print_report(sources.len(), total_bytes, max_lines);
}

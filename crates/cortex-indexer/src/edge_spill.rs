//! Append-only edge storage for large-repo indexing.
//!
//! Avoids holding the full `Vec<CodeEdge>` for the whole repository in memory.

use cortex_core::CodeNode;
use cortex_core::{CodeEdge, CortexError, EdgeKind, IndexedFile, Language, Result};
use cortex_graph::EdgeWriteProfile;
use std::collections::HashSet;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::time::Instant;
use tempfile::tempfile;

/// Versioned binary edge line tag (MessagePack payload); legacy lines are JSON.
const EDGE_SPILL_RMP_TAG: u8 = 0x01;

/// Breakdown of edge spill flush when profiling is enabled.
#[derive(Debug, Clone, Default)]
pub(crate) struct EdgeSpillFlushDetail {
    pub read_secs: f64,
    pub bolt_secs: f64,
}

/// On-disk newline-delimited edges plus a set of `call_target:*` ids for resolution.
pub(crate) struct EdgeSpill {
    writer: BufWriter<std::fs::File>,
    edge_count: u64,
    call_target_ids: HashSet<String>,
}

impl EdgeSpill {
    pub(crate) fn new() -> Result<Self> {
        let f = tempfile().map_err(|e| CortexError::Io(e.to_string()))?;
        Ok(Self {
            writer: BufWriter::new(f),
            edge_count: 0,
            call_target_ids: HashSet::new(),
        })
    }

    pub(crate) fn push(&mut self, edge: &CodeEdge) -> Result<()> {
        let payload = rmp_serde::to_vec(edge).map_err(|e| CortexError::Io(e.to_string()))?;
        let len = u32::try_from(payload.len())
            .map_err(|_| CortexError::Io("edge spill payload exceeds u32::MAX".to_string()))?;
        self.writer
            .write_all(&[EDGE_SPILL_RMP_TAG])
            .map_err(|e| CortexError::Io(e.to_string()))?;
        self.writer
            .write_all(&len.to_le_bytes())
            .map_err(|e| CortexError::Io(e.to_string()))?;
        self.writer
            .write_all(&payload)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        self.writer
            .write_all(b"\n")
            .map_err(|e| CortexError::Io(e.to_string()))?;
        self.edge_count += 1;
        if edge.to.starts_with("call_target:")
            && matches!(
                edge.kind,
                EdgeKind::Calls | EdgeKind::TypeReference | EdgeKind::FieldAccess
            )
        {
            self.call_target_ids.insert(edge.to.clone());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn edge_count(&self) -> u64 {
        self.edge_count
    }

    /// Drain call-target ids into `(id, name)` pairs (one owned pass; spill remains for flush).
    pub(crate) fn take_call_target_pairs(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.call_target_ids)
            .into_iter()
            .map(|id| {
                let name = id.trim_start_matches("call_target:").to_string();
                (id, name)
            })
            .collect()
    }

    /// Flush and stream edges in chunks to `write_edges`.
    ///
    /// Returns `(edges_written, bolt_write_executions, optional_read/bolt_split)`.
    pub(crate) async fn stream_to_writer(
        mut self,
        writer: &cortex_graph::NodeWriter,
        chunk_size: usize,
        mut profile: Option<&mut EdgeWriteProfile>,
    ) -> Result<(u64, u64, Option<EdgeSpillFlushDetail>)> {
        self.writer
            .flush()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        let mut file = self
            .writer
            .into_inner()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        file.flush().map_err(|e| CortexError::Io(e.to_string()))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| CortexError::Io(e.to_string()))?;

        let mut reader = file;
        let mut chunk: Vec<CodeEdge> = Vec::with_capacity(chunk_size.max(1));
        let mut written: u64 = 0;
        let mut bolt_executions: u64 = 0;
        let profile_on = profile.is_some();
        let mut read_time = std::time::Duration::ZERO;
        let mut bolt_time = std::time::Duration::ZERO;

        loop {
            let t_read = Instant::now();
            let edge = match read_spill_edge_line(&mut reader)? {
                Some(e) => e,
                None => break,
            };
            if profile_on {
                read_time += t_read.elapsed();
            }
            chunk.push(edge);
            if chunk.len() >= chunk_size.max(1) {
                let t_bolt = Instant::now();
                bolt_executions += writer
                    .write_edges_profiled(&chunk, profile.as_deref_mut())
                    .await?;
                if profile_on {
                    bolt_time += t_bolt.elapsed();
                }
                written += chunk.len() as u64;
                chunk.clear();
            }
        }
        if !chunk.is_empty() {
            let t_bolt = Instant::now();
            bolt_executions += writer
                .write_edges_profiled(&chunk, profile.as_deref_mut())
                .await?;
            if profile_on {
                bolt_time += t_bolt.elapsed();
            }
            written += chunk.len() as u64;
            chunk.clear();
        }

        let detail = if profile_on {
            Some(EdgeSpillFlushDetail {
                read_secs: read_time.as_secs_f64(),
                bolt_secs: bolt_time.as_secs_f64(),
            })
        } else {
            None
        };
        Ok((written, bolt_executions, detail))
    }
}

fn read_spill_edge_line(reader: &mut impl Read) -> Result<Option<CodeEdge>> {
    let mut tag = [0u8; 1];
    match reader.read_exact(&mut tag) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(CortexError::Io(e.to_string())),
    }

    if tag[0] == EDGE_SPILL_RMP_TAG {
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        reader
            .read_exact(&mut payload)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        let mut tail = [0u8; 1];
        reader
            .read_exact(&mut tail)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        if tail[0] != b'\n' {
            return Err(CortexError::Io(
                "edge spill binary record missing trailing newline".to_string(),
            ));
        }
        let edge: CodeEdge =
            rmp_serde::from_slice(&payload).map_err(|e| CortexError::Io(e.to_string()))?;
        return Ok(Some(edge));
    }

    let mut line = vec![tag[0]];
    let mut ch = [0u8; 1];
    loop {
        reader
            .read_exact(&mut ch)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        if ch[0] == b'\n' {
            break;
        }
        line.push(ch[0]);
    }
    let edge: CodeEdge =
        serde_json::from_slice(&line).map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(Some(edge))
}

/// Slim per-file spill row for deferred node replay.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DeferredFileRecord {
    pub path: String,
    pub language: Language,
    pub content_hash: String,
    /// MessagePack-encoded nodes (preferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes_rmp: Option<Vec<u8>>,
    /// Legacy JSON inline nodes (empty when `nodes_rmp` is set).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<CodeNode>,
}

impl DeferredFileRecord {
    pub(crate) fn nodes(&self) -> Result<Vec<CodeNode>> {
        if let Some(blob) = &self.nodes_rmp {
            return rmp_serde::from_read(blob.as_slice())
                .map_err(|e| CortexError::Io(e.to_string()));
        }
        Ok(self.nodes.clone())
    }
}

impl From<&IndexedFile> for DeferredFileRecord {
    fn from(file: &IndexedFile) -> Self {
        let nodes_rmp = rmp_serde::to_vec(&file.nodes).ok();
        Self {
            path: file.path.clone(),
            language: file.language,
            content_hash: file.content_hash.clone(),
            nodes_rmp,
            nodes: Vec::new(),
        }
    }
}

/// NDJSON spill of [`DeferredFileRecord`] for forced branch rebuilds without holding the full repo in RAM.
pub(crate) struct DeferredIndexedSpill {
    writer: BufWriter<std::fs::File>,
    file_count: usize,
    bytes_written: u64,
}

impl DeferredIndexedSpill {
    pub(crate) fn new() -> Result<Self> {
        let f = tempfile().map_err(|e| CortexError::Io(e.to_string()))?;
        Ok(Self {
            writer: BufWriter::new(f),
            file_count: 0,
            bytes_written: 0,
        })
    }

    pub(crate) fn push(&mut self, file: &IndexedFile) -> Result<()> {
        let record = DeferredFileRecord::from(file);
        let mut line = Vec::new();
        serde_json::to_writer(&mut line, &record).map_err(|e| CortexError::Io(e.to_string()))?;
        line.push(b'\n');
        self.bytes_written += line.len() as u64;
        self.writer
            .write_all(&line)
            .map_err(|e| CortexError::Io(e.to_string()))?;
        self.file_count += 1;
        Ok(())
    }

    pub(crate) fn spill_bytes(&self) -> u64 {
        self.bytes_written
    }

    #[allow(dead_code)]
    pub(crate) fn file_count(&self) -> usize {
        self.file_count
    }

    pub(crate) fn into_buffered_reader(mut self) -> Result<BufReader<std::fs::File>> {
        self.writer
            .flush()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        let mut file = self
            .writer
            .into_inner()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        file.flush().map_err(|e| CortexError::Io(e.to_string()))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| CortexError::Io(e.to_string()))?;
        Ok(BufReader::new(file))
    }
}

/// Deserialize a deferred spill line (new slim format or legacy full [`IndexedFile`]).
pub(crate) fn parse_deferred_spill_line(line: &str) -> Result<DeferredFileRecord> {
    if let Ok(record) = serde_json::from_str::<DeferredFileRecord>(line) {
        return Ok(record);
    }
    let legacy: IndexedFile =
        serde_json::from_str(line).map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(DeferredFileRecord::from(&legacy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::EdgeKind;
    use std::collections::HashMap;

    fn sample_edge(from: &str, to: &str, kind: EdgeKind) -> CodeEdge {
        CodeEdge {
            from: from.to_string(),
            to: to.to_string(),
            kind,
            properties: HashMap::new(),
        }
    }

    #[test]
    fn edge_spill_rmp_roundtrip() {
        let mut spill = EdgeSpill::new().unwrap();
        spill.push(&sample_edge("a", "b", EdgeKind::Calls)).unwrap();
        spill
            .push(&sample_edge("b", "c", EdgeKind::Contains))
            .unwrap();
        assert_eq!(spill.edge_count(), 2);
        spill.writer.flush().unwrap();
        let mut file = spill.writer.into_inner().unwrap();
        file.sync_all().unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = file;
        let e1 = read_spill_edge_line(&mut reader).unwrap().unwrap();
        let e2 = read_spill_edge_line(&mut reader).unwrap().unwrap();
        assert_eq!(e1.from, "a");
        assert_eq!(e2.kind, EdgeKind::Contains);
        assert!(read_spill_edge_line(&mut reader).unwrap().is_none());
    }

    #[test]
    fn edge_spill_legacy_json_line() {
        let edge = sample_edge("x", "y", EdgeKind::Imports);
        let mut line = serde_json::to_vec(&edge).unwrap();
        line.push(b'\n');
        let mut reader = line.as_slice();
        let decoded = read_spill_edge_line(&mut reader).unwrap().unwrap();
        assert_eq!(decoded.from, "x");
    }

    #[test]
    fn deferred_record_rmp_nodes() {
        let file = IndexedFile {
            path: "src/a.rs".to_string(),
            language: Language::Rust,
            content_hash: "h".to_string(),
            nodes: vec![CodeNode {
                id: "n1".to_string(),
                kind: cortex_core::EntityKind::Function,
                name: "foo".to_string(),
                path: Some("src/a.rs".to_string()),
                line_number: Some(1),
                lang: Some(Language::Rust),
                source: None,
                docstring: None,
                properties: HashMap::new(),
            }],
            edges: Vec::new(),
        };
        let rec = DeferredFileRecord::from(&file);
        assert!(rec.nodes_rmp.is_some());
        assert!(rec.nodes.is_empty());
        assert_eq!(rec.nodes().unwrap().len(), 1);
    }
}

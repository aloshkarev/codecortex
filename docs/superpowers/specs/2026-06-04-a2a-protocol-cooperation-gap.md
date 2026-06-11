# A2A Protocol-Native MCP Cooperation Gap Memo (Phase 3 Batch 0)

**Date:** 2026-06-04  
**References:** [a2a.proto](../../a2a.proto), [specification.md](../../specification.md)

## Method

Static trace of wire codec, hub streaming, and session artifact mapping. Phase 2 logical cooperation (IntelligencePack, spawn hints) is complete; this memo covers **spec-shaped** gaps.

## Proto / spec vs implementation

| Spec field | Proto / spec ref | Current implementation | Gap |
| --- | --- | --- | --- |
| `Artifact.artifact_id` | proto L280 | `artifact-{n}` sequential in [`session.rs:69`](../../crates/cortex-a2a/src/session.rs) | No stable `{task_id}/{kind}/{tool}` ids |
| `Artifact.name` | proto L283 | Always `codecortex.result` | No per-kind names |
| `Artifact.metadata` | proto L290 | Always `None` on wire | `mcpToolId`, freshness, hints only inside `Part.data` |
| `Artifact.extensions` | proto L292 | Not on `ArtifactWire` | Missing intelligence-cooperation extension URI |
| `Part.data` | proto L232 | Used in `to_wire` | OK for HTTP JSON path |
| `Part.data` gRPC round-trip | proto L232 | [`spec_codec.rs:176-182`](../../crates/cortex-a2a/src/spec_codec.rs) drops data on `task_proto_to_wire` | **High** — gRPC clients lose intelligence JSON |
| `Task.metadata` | proto L183 | Hardcoded `None` in codec L134 | No workflow/scope/hints at task level |
| `TaskArtifactUpdateEvent` | proto L308 | `emit_task_wire` sets `artifact_update: None` [`hub.rs:276`](../../crates/cortex-a2a/src/hub.rs) | **High** — no incremental artifact stream |
| `stream_wire_to_proto` artifact branch | proto L800 | Only status + task [`spec_codec.rs:236-258`](../../crates/cortex-a2a/src/spec_codec.rs) | Artifact updates not encoded for gRPC |
| `GetTask includeArtifacts` | spec §2.3 | Not on MCP `A2aGetTaskReq` | Cannot omit artifacts per spec |
| `AgentSkill` MCP tools | proto L435 | Generic tags in [`agent_card.rs`](../../crates/cortex-a2a/src/agent_card.rs) | `mcp_tools` manifest not on card |
| `AgentExtension` cooperation | spec §4.6 | Blackboard URI only [`wire.rs:6`](../../crates/cortex-a2a/src/wire.rs) | No intelligence-cooperation extension |

## Phase 3 batch mapping

| Batch | Closes |
| --- | --- |
| 1 | CooperationArtifactBuilder, spec ArtifactWire |
| 2 | Task.metadata, includeArtifacts |
| 3 | TaskArtifactUpdateEvent streaming |
| 4 | AgentSkill + extension registration |
| 5 | spec_codec data Part parity, handler IntelligencePack migration |
| 6 | Oracles, audit, docs pillar |

## Exit criteria

Field-level table with file/line references — **yes**.

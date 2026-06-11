//! Thin re-exports over the shared [`crate::intelligence`] module (legacy host helpers).

pub use crate::intelligence::{
    ImpactGraphParams, PatchContextParams, ScopeFilters, compute_api_contract,
    compute_delta_context, compute_impact_graph, compute_patch_context, compute_test_context,
    impact_summary_for_a2a, patch_capsule_from_data, resolve_symbol,
};

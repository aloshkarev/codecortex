//! # Cortex Pipeline
//!
//! ECL (Extract → Cognify → Load) Pipeline for structured code processing.
//!
//! This crate provides a flexible pipeline architecture inspired by [cognee](https://github.com/topoteretes/cognee)
//! for processing code through multiple enrichment stages.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                        Pipeline                                    │
//! ├────────────────┬────────────────┬────────────────┬───────────────┤
//! │    Extract     │    Cognify     │     Embed      │     Load      │
//! │                │                │                │               │
//! │ • Parse files  │ • Extract rel. │ • Generate     │ • Store in    │
//! │ • Detect lang  │ • Calc metrics │   embeddings   │   graph +     │
//! │ • Build AST    │ • Identify sm. │ • Summarize    │   vector DB   │
//! └────────────────┴────────────────┴────────────────┴───────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cortex_pipeline::{Pipeline, PipelineContext, stages::*};
//!
//! // Create pipeline with default stages
//! let mut pipeline = Pipeline::new()
//!     .add_stage(ExtractStage::new(parser_registry))
//!     .add_stage(CognifyStage::new())
//!     .add_stage(EmbedStage::new(embedder))
//!     .add_stage(LoadStage::new(graph_client, vector_store));
//!
//! // Process input
//! let context = PipelineContext::from_path("/path/to/code");
//! let result = pipeline.run(context).await?;
//! ```

pub mod context;
pub mod pipeline;
pub mod stage;

pub use context::PipelineContext;
pub use pipeline::Pipeline;
pub use stage::{CognifyStage, EmbedStage, ExtractStage, LoadStage, Stage, StageResult};

//! Pipeline runner for orchestrating stages.
//!
//! The Pipeline struct manages stage execution and error handling.

use crate::context::PipelineContext;
use crate::stage::Stage;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{Level, info, instrument, span};

/// Pipeline for processing code through multiple stages.
///
/// Stages are executed in order (Extract → Cognify → Embed → Load).
/// Each stage transforms the context for the next stage.
pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
    /// Shared state for progress tracking
    state: Arc<RwLock<PipelineState>>,
}

/// Current state of the pipeline
#[derive(Debug, Clone, Default)]
pub struct PipelineState {
    /// Current stage being executed
    pub current_stage: Option<String>,
    /// Completed stages
    pub completed_stages: Vec<String>,
    /// Number of entities processed
    pub entities_processed: usize,
    /// Has the pipeline finished?
    pub is_complete: bool,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl Pipeline {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            state: Arc::new(RwLock::new(PipelineState::default())),
        }
    }

    /// Add a stage to the pipeline
    pub fn add_stage<S: Stage + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Arc::new(stage));
        self
    }

    /// Get current pipeline state
    pub async fn state(&self) -> PipelineState {
        self.state.read().await.clone()
    }

    /// Run the pipeline with the given context
    #[instrument(skip(self, context), fields(stages = self.stages.len()))]
    pub async fn run(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        let span = span!(Level::INFO, "pipeline_run");
        let _enter = span.enter();

        {
            let mut state = self.state.write().await;
            state.current_stage = None;
            state.completed_stages.clear();
            state.entities_processed = 0;
            state.is_complete = false;
            state.errors.clear();
        }

        info!(stages = self.stages.len(), "Starting pipeline execution");

        for stage in &self.stages {
            let stage_name = stage.name();

            // Update state
            {
                let mut state = self.state.write().await;
                state.current_stage = Some(stage_name.to_string());
            }

            info!(stage = stage_name, "Executing stage");

            // Execute stage
            match stage.process(&mut context).await {
                Ok(result) => {
                    info!(
                        stage = stage_name,
                        processed = result.processed_count,
                        skipped = result.skipped_count,
                        "Stage completed"
                    );

                    // Update state
                    let mut state = self.state.write().await;
                    state.completed_stages.push(stage_name.to_string());
                    state.entities_processed += result.processed_count;

                    // Record warnings
                    for warning in result.warnings {
                        tracing::warn!(stage = stage_name, warning = %warning);
                    }
                }
                Err(e) => {
                    tracing::error!(stage = stage_name, error = %e, "Stage failed");

                    let mut state = self.state.write().await;
                    state.errors.push(format!("{}: {}", stage_name, e));
                    state.current_stage = None;
                    return Err(e);
                }
            }
        }

        // Mark complete
        {
            let mut state = self.state.write().await;
            state.is_complete = true;
            state.current_stage = None;
        }

        info!("Pipeline execution complete");
        Ok(context)
    }

    /// Run the pipeline with default ECL stages
    pub fn with_default_stages() -> Self {
        Self::new()
            .add_stage(crate::stage::ExtractStage::new())
            .add_stage(crate::stage::CognifyStage::new())
            .add_stage(crate::stage::EmbedStage::new())
            .add_stage(crate::stage::LoadStage::new())
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_creation() {
        let pipeline = Pipeline::new();
        assert!(pipeline.stages.is_empty());
    }

    #[test]
    fn pipeline_with_default_stages() {
        let pipeline = Pipeline::with_default_stages();
        assert_eq!(pipeline.stages.len(), 4);
    }

    #[tokio::test]
    async fn pipeline_state_tracking() {
        let pipeline = Pipeline::with_default_stages();
        let initial_state = pipeline.state().await;
        assert!(initial_state.current_stage.is_none());
        assert!(initial_state.completed_stages.is_empty());
    }

    #[tokio::test]
    async fn pipeline_run_with_content() {
        let pipeline = Pipeline::with_default_stages();
        let context = crate::context::PipelineContext::from_content(
            "test.rs".to_string(),
            "fn main() {}".to_string(),
            Some("rs".to_string()),
        );

        let result = pipeline.run(context).await;
        assert!(result.is_ok());

        let final_context = result.unwrap();
        assert!(final_context.complete);

        let state = pipeline.state().await;
        assert!(state.is_complete);
        assert_eq!(state.completed_stages.len(), 4);
    }

    #[tokio::test]
    async fn pipeline_state_resets_between_runs() {
        let pipeline = Pipeline::with_default_stages();
        let context1 = crate::context::PipelineContext::from_content(
            "test.rs".to_string(),
            "fn main() {}".to_string(),
            Some("rs".to_string()),
        );
        let context2 = crate::context::PipelineContext::from_content(
            "other.rs".to_string(),
            "fn helper() {}".to_string(),
            Some("rs".to_string()),
        );

        pipeline
            .run(context1)
            .await
            .expect("first run should succeed");
        let state_after_first = pipeline.state().await;
        assert_eq!(state_after_first.completed_stages.len(), 4);

        pipeline
            .run(context2)
            .await
            .expect("second run should succeed");
        let state_after_second = pipeline.state().await;
        assert_eq!(state_after_second.completed_stages.len(), 4);
        assert_eq!(state_after_second.entities_processed, 4);
        assert!(state_after_second.errors.is_empty());
    }
}

//! Schema Migrations for Graph Database
//!
//! This module provides a versioned migration system for database schema changes.
//! Migrations are tracked in the database and only run once.

use crate::GraphClient;
use cortex_core::Result;
use std::time::Instant;

/// Migration version type
pub type MigrationVersion = u64;

/// A single migration definition
#[derive(Debug, Clone)]
pub struct Migration {
    /// Version number (must be unique and sequential)
    pub version: MigrationVersion,
    /// Human-readable name
    pub name: &'static str,
    /// SQL/Cypher statements to execute
    pub statements: &'static [&'static str],
}

/// Current schema version
pub const CURRENT_VERSION: MigrationVersion = 1;

/// All migrations in order
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        statements: &[
            // Constraints
            "CREATE CONSTRAINT ON (r:Repository) ASSERT r.path IS UNIQUE;",
            "CREATE CONSTRAINT ON (d:Directory) ASSERT d.path IS UNIQUE;",
            "CREATE CONSTRAINT ON (f:File) ASSERT f.path IS UNIQUE;",
            // Indexes for common queries
            "CREATE INDEX ON :Function(name);",
            "CREATE INDEX ON :Function(path);",
            "CREATE INDEX ON :Class(name);",
            "CREATE INDEX ON :Class(path);",
            "CREATE INDEX ON :Variable(name);",
            "CREATE INDEX ON :Parameter(name);",
            "CREATE INDEX ON :Module(name);",
            "CREATE INDEX ON :CallTarget(name);",
            "CREATE INDEX ON :CodeNode(path);",
            "CREATE INDEX ON :CodeNode(kind);",
            "CREATE INDEX ON :CodeNode(name);",
            "CREATE INDEX ON :CodeNode(line_number);",
        ],
    },
];

/// Migration result
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Version applied
    pub version: MigrationVersion,
    /// Name of the migration
    pub name: String,
    /// Whether the migration was applied (false if already existed)
    pub applied: bool,
    /// Time taken to apply
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Migration manager for tracking and applying migrations
pub struct MigrationManager<'a> {
    client: &'a GraphClient,
}

impl<'a> MigrationManager<'a> {
    /// Create a new migration manager
    pub fn new(client: &'a GraphClient) -> Self {
        Self { client }
    }

    /// Get the current schema version from the database
    pub async fn current_version(&self) -> Result<MigrationVersion> {
        // Ensure migration tracking node exists
        self.ensure_migration_node().await?;

        let result = self
            .client
            .raw_query("MATCH (m:SchemaMigration {id: 'schema_version'}) RETURN m.version AS version")
            .await?;

        if let Some(row) = result.first()
            && let Some(version) = row.get("version").and_then(|v| v.as_u64())
        {
            return Ok(version);
        }

        Ok(0)
    }

    /// Ensure the migration tracking node exists
    async fn ensure_migration_node(&self) -> Result<()> {
        self.client
            .run(
                "MERGE (m:SchemaMigration {id: 'schema_version'}) \
                 ON CREATE SET m.version = 0, m.created_at = timestamp()",
            )
            .await
    }

    /// Apply all pending migrations
    pub async fn apply_all(&self) -> Result<Vec<MigrationResult>> {
        let mut results = Vec::new();
        let current = self.current_version().await?;

        for migration in MIGRATIONS {
            if migration.version > current {
                let result = self.apply_migration(migration).await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Apply a single migration
    pub async fn apply_migration(&self, migration: &Migration) -> Result<MigrationResult> {
        let start = Instant::now();
        let mut applied = false;
        let mut error = None;

        // Check if already applied
        let current = self.current_version().await?;
        if migration.version <= current {
            return Ok(MigrationResult {
                version: migration.version,
                name: migration.name.to_string(),
                applied: false,
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            });
        }

        // Apply statements
        for statement in migration.statements {
            if let Err(e) = self.client.run(statement).await {
                // Ignore "already exists" errors for idempotency
                if !e.to_string().contains("already exists")
                    && !e.to_string().contains("ConstraintAlreadyExists")
                    && !e.to_string().contains("IndexAlreadyExists")
                {
                    error = Some(e.to_string());
                    break;
                }
            }
        }

        if error.is_none() {
            // Update version
            let update_result = self
                .client
                .run(&format!(
                    "MATCH (m:SchemaMigration {{id: 'schema_version'}}) SET m.version = {}, m.last_applied = timestamp()",
                    migration.version
                ))
                .await;

            if update_result.is_err() {
                error = update_result.err().map(|e| e.to_string());
            } else {
                applied = true;
            }
        }

        Ok(MigrationResult {
            version: migration.version,
            name: migration.name.to_string(),
            applied,
            duration_ms: start.elapsed().as_millis() as u64,
            error,
        })
    }

    /// Get list of applied migrations
    pub async fn applied_migrations(&self) -> Result<Vec<MigrationVersion>> {
        let result = self
            .client
            .raw_query(
                "MATCH (m:SchemaMigration {id: 'schema_version'}) RETURN m.version AS version",
            )
            .await?;

        let current = if let Some(row) = result.first() {
            row.get("version").and_then(|v| v.as_u64()).unwrap_or(0)
        } else {
            0
        };

        Ok(MIGRATIONS.iter().filter(|m| m.version <= current).map(|m| m.version).collect())
    }

    /// Check if there are pending migrations
    pub async fn has_pending_migrations(&self) -> Result<bool> {
        let current = self.current_version().await?;
        Ok(MIGRATIONS.iter().any(|m| m.version > current))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_sequential() {
        let versions: Vec<u64> = MIGRATIONS.iter().map(|m| m.version).collect();
        for (i, v) in versions.iter().enumerate() {
            assert_eq!(*v, (i + 1) as u64, "Migration versions should be sequential");
        }
    }

    #[test]
    fn current_version_matches_last_migration() {
        let max_version = MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0);
        assert_eq!(CURRENT_VERSION, max_version);
    }

    #[test]
    fn migrations_have_names() {
        for migration in MIGRATIONS {
            assert!(!migration.name.is_empty(), "Migration v{} needs a name", migration.version);
        }
    }

    #[test]
    fn migrations_have_statements() {
        for migration in MIGRATIONS {
            assert!(!migration.statements.is_empty(), "Migration v{} has no statements", migration.version);
        }
    }

    #[test]
    fn migration_statements_end_with_semicolon() {
        for migration in MIGRATIONS {
            for statement in migration.statements {
                assert!(statement.ends_with(';'), "Statement doesn't end with semicolon: {}", statement);
            }
        }
    }

    #[test]
    fn initial_migration_has_constraints() {
        let initial = MIGRATIONS.iter().find(|m| m.version == 1);
        assert!(initial.is_some());

        let initial = initial.unwrap();
        let constraint_count = initial.statements.iter().filter(|s| s.contains("CONSTRAINT")).count();
        assert!(constraint_count >= 3, "Initial migration should have at least 3 constraints");
    }

    #[test]
    fn initial_migration_has_indexes() {
        let initial = MIGRATIONS.iter().find(|m| m.version == 1);
        assert!(initial.is_some());

        let initial = initial.unwrap();
        let index_count = initial.statements.iter().filter(|s| s.contains("INDEX")).count();
        assert!(index_count >= 10, "Initial migration should have at least 10 indexes");
    }
}

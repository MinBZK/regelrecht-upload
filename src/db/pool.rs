//! Database connection pool

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create a new database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    tracing::info!("Creating database pool...");

    // Retry connection with backoff
    let mut attempts = 0;
    let max_attempts = 5;

    loop {
        attempts += 1;
        tracing::info!("Database connection attempt {}/{}", attempts, max_attempts);

        let result = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await;

        match result {
            Ok(pool) => {
                tracing::info!("Database pool created successfully");
                return Ok(pool);
            }
            Err(e) => {
                if attempts >= max_attempts {
                    tracing::error!(
                        "Failed to connect to database after {} attempts: {}",
                        attempts,
                        e
                    );
                    return Err(e);
                }
                tracing::warn!(
                    "Database connection failed (attempt {}): {}, retrying in {}s...",
                    attempts,
                    e,
                    attempts * 2
                );
                tokio::time::sleep(Duration::from_secs(attempts as u64 * 2)).await;
            }
        }
    }
}

/// Split SQL into statements, properly handling $$ delimited blocks (PL/pgSQL functions)
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_dollar_block = false;
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        current.push(c);

        // Check for $$ delimiter
        if c == '$' && i + 1 < chars.len() && chars[i + 1] == '$' {
            current.push(chars[i + 1]);
            i += 1;
            in_dollar_block = !in_dollar_block;
        }
        // Check for statement end (semicolon outside of $$ block)
        else if c == ';' && !in_dollar_block {
            let trimmed = current.trim();
            if !trimmed.is_empty() && has_sql_content(trimmed) {
                statements.push(current.clone());
            }
            current.clear();
        }

        i += 1;
    }

    // Handle any remaining content
    let trimmed = current.trim();
    if !trimmed.is_empty() && has_sql_content(trimmed) {
        statements.push(current);
    }

    statements
}

/// Check if a string has actual SQL content (not just comments)
fn has_sql_content(s: &str) -> bool {
    s.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with("--")
    })
}

/// Run database migrations with tracking
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create migrations tracking table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    // Handle legacy databases: if schema exists but wasn't tracked, mark as applied
    // This prevents re-running 001_initial on servers where it already ran
    let submissions_exists: Option<(String,)> = sqlx::query_as(
        "SELECT table_name::text FROM information_schema.tables
         WHERE table_schema = 'public' AND table_name = 'submissions'"
    )
    .fetch_optional(pool)
    .await?;

    if submissions_exists.is_some() {
        // Schema exists - ensure 001_initial is marked as applied
        sqlx::query(
            "INSERT INTO _migrations (name) VALUES ('001_initial')
             ON CONFLICT (name) DO NOTHING"
        )
        .execute(pool)
        .await?;
        tracing::info!("Legacy schema detected, marked 001_initial as applied");
    }

    // Define all migrations in order
    let migrations = [
        ("001_initial", include_str!("migrations/001_initial.sql")),
        ("003_retention_date", include_str!("migrations/003_retention_date.sql")),
        ("004_uploader_sessions", include_str!("migrations/004_uploader_sessions.sql")),
    ];

    for (name, sql) in migrations {
        // Check if already applied
        let already_applied: Option<(String,)> =
            sqlx::query_as("SELECT name FROM _migrations WHERE name = $1")
                .bind(name)
                .fetch_optional(pool)
                .await?;

        if already_applied.is_some() {
            tracing::debug!("Migration {} already applied, skipping", name);
            continue;
        }

        tracing::info!("Applying migration: {}", name);

        // Split and execute statements
        let statements = split_sql_statements(sql);
        for statement in &statements {
            sqlx::query(statement).execute(pool).await.map_err(|e| {
                tracing::error!("Migration {} failed: {}", name, e);
                e
            })?;
        }

        // Record as applied
        sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
            .bind(name)
            .execute(pool)
            .await?;

        tracing::info!("Migration {} applied successfully", name);
    }

    Ok(())
}

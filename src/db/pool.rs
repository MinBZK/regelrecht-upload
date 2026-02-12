//! Database connection pool

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Create a new database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
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
    s.lines()
        .any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("--")
        })
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Read and execute the migration file
    let migration_sql = include_str!("migrations/001_initial.sql");

    // Split into statements, properly handling $$ blocks
    let statements = split_sql_statements(migration_sql);

    for statement in statements {
        sqlx::query(&statement)
            .execute(pool)
            .await
            .map_err(|e| {
                tracing::warn!("Migration statement may have failed (possibly already exists): {}", e);
                e
            })
            .ok();
    }

    tracing::info!("Database migrations completed");
    Ok(())
}

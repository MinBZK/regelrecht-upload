//! Application configuration
//!
//! Loads configuration from environment variables with sensible defaults.

use std::env;

/// Application configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Database connection URL
    pub database_url: String,
    /// Upload directory path
    pub upload_dir: String,
    /// Frontend assets directory
    pub frontend_dir: String,
    /// Session expiration in hours
    pub session_expiry_hours: u64,
    /// Maximum upload file size in bytes
    pub max_upload_size: usize,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Environment (development/production)
    pub environment: Environment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let environment = match env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        };

        let database_url = env::var("DATABASE_URL").map_err(|_| {
            ConfigError::Missing("DATABASE_URL is required".to_string())
        })?;

        Ok(Config {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url,
            upload_dir: env::var("UPLOAD_DIR")
                .unwrap_or_else(|_| "/app/uploads".to_string()),
            frontend_dir: env::var("FRONTEND_DIR")
                .unwrap_or_else(|_| "./frontend".to_string()),
            session_expiry_hours: env::var("SESSION_EXPIRY_HOURS")
                .ok()
                .and_then(|h| h.parse().ok())
                .unwrap_or(8),
            max_upload_size: env::var("MAX_UPLOAD_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50 * 1024 * 1024), // 50MB default
            cors_origins: env::var("CORS_ORIGINS")
                .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["http://localhost:8080".to_string()]),
            environment,
        })
    }

    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    /// Get the server address
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ConfigError {
    #[error("Missing configuration: {0}")]
    Missing(String),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

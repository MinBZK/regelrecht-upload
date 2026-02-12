//! Authentication handlers

use crate::models::*;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

use super::AppState;

/// Session cookie name
pub const SESSION_COOKIE: &str = "rr_admin_session";

/// Rate limit: max attempts per IP per hour
const MAX_LOGIN_ATTEMPTS: i64 = 10;

// =============================================================================
// Authentication Trait (for future SSO/OAuth extension)
// =============================================================================

/// Authentication provider trait for modular auth backends
#[allow(dead_code)]
pub trait AuthProvider: Send + Sync {
    fn authenticate(&self, username: &str, password: &str) -> impl std::future::Future<Output = Option<Uuid>> + Send;
    fn validate_session(&self, token: &str) -> impl std::future::Future<Output = Option<AdminUser>> + Send;
}

// =============================================================================
// Login Endpoint
// =============================================================================

/// Admin login
pub async fn admin_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers);

    // Check rate limit
    if !check_rate_limit(&state.pool, &client_ip, "login").await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::<AdminUserResponse>::error(
                "Too many login attempts. Please try again later.",
            )),
        );
    }

    // Record attempt
    record_attempt(&state.pool, &client_ip, "login").await;

    // Find user
    let user = sqlx::query_as::<_, AdminUser>(
        "SELECT * FROM admin_users WHERE username = $1 AND is_active = true",
    )
    .bind(&input.username)
    .fetch_optional(&state.pool)
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        Ok(None) | Err(_) => {
            // Don't reveal whether username exists
            return (
                StatusCode::UNAUTHORIZED,
                [(header::SET_COOKIE, "".to_string())],
                Json(ApiResponse::error("Invalid username or password")),
            );
        }
    };

    // Verify password
    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(h) => h,
        Err(_) => {
            tracing::error!("Invalid password hash in database for user {}", user.username);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::SET_COOKIE, "".to_string())],
                Json(ApiResponse::error("Authentication error")),
            );
        }
    };

    if Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::error("Invalid username or password")),
        );
    }

    // Generate session token
    let token = generate_session_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(8);

    // Create session
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(500).collect::<String>());

    let session_result = sqlx::query(
        r#"
        INSERT INTO admin_sessions (admin_user_id, token_hash, expires_at, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .bind(&client_ip)
    .bind(&user_agent)
    .execute(&state.pool)
    .await;

    if session_result.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::error("Failed to create session")),
        );
    }

    // Update last login
    let _ = sqlx::query("UPDATE admin_users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.pool)
        .await;

    // Log audit event
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id, actor_ip)
        VALUES ('admin_login'::audit_action, 'admin_user', $1, 'admin', $1, $2)
        "#,
    )
    .bind(user.id)
    .bind(&client_ip)
    .execute(&state.pool)
    .await;

    // Set secure cookie
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        SESSION_COOKIE,
        token,
        8 * 3600 // 8 hours
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::success(AdminUserResponse::from(user))),
    )
}

/// Admin logout
pub async fn admin_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_session_token(&headers);

    if let Some(token) = token {
        let token_hash = hash_token(&token);

        // Get session for audit log
        let session = sqlx::query_as::<_, AdminSession>(
            "SELECT * FROM admin_sessions WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        // Delete session
        let _ = sqlx::query("DELETE FROM admin_sessions WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(&state.pool)
            .await;

        // Log audit event
        if let Some(session) = session {
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id)
                VALUES ('admin_logout'::audit_action, 'admin_user', $1, 'admin', $1)
                "#,
            )
            .bind(session.admin_user_id)
            .execute(&state.pool)
            .await;
        }
    }

    // Clear cookie
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        SESSION_COOKIE
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::success(())),
    )
}

/// Get current admin user
pub async fn get_current_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match validate_admin_session(&state.pool, &headers).await {
        Some(user) => (StatusCode::OK, Json(ApiResponse::success(AdminUserResponse::from(user)))),
        None => (StatusCode::UNAUTHORIZED, Json(ApiResponse::error("Not authenticated"))),
    }
}

// =============================================================================
// Session Validation
// =============================================================================

/// Validate admin session from headers
pub async fn validate_admin_session(pool: &PgPool, headers: &HeaderMap) -> Option<AdminUser> {
    let token = extract_session_token(headers)?;
    let token_hash = hash_token(&token);

    // Find valid session
    let session = sqlx::query_as::<_, AdminSession>(
        r#"
        SELECT * FROM admin_sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .ok()??;

    // Get associated user
    sqlx::query_as::<_, AdminUser>(
        "SELECT * FROM admin_users WHERE id = $1 AND is_active = true",
    )
    .bind(session.admin_user_id)
    .fetch_optional(pool)
    .await
    .ok()?
}

// =============================================================================
// Password Utilities
// =============================================================================

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Create an admin user (utility function for setup)
#[allow(dead_code)]
pub async fn create_admin_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    password: &str,
    display_name: Option<&str>,
) -> Result<AdminUser, sqlx::Error> {
    let password_hash = hash_password(password)
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    sqlx::query_as::<_, AdminUser>(
        r#"
        INSERT INTO admin_users (username, email, password_hash, display_name)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .fetch_one(pool)
    .await
}

// =============================================================================
// Helper Functions
// =============================================================================

fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE)) {
            return Some(value.to_string());
        }
    }

    None
}

fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn get_client_ip(headers: &HeaderMap) -> String {
    // Check X-Forwarded-For first (for reverse proxy setups)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first_ip) = xff_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }

    "unknown".to_string()
}

async fn check_rate_limit(pool: &PgPool, ip: &str, endpoint: &str) -> bool {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM rate_limit_attempts
        WHERE ip_address = $1 AND endpoint = $2
        AND attempted_at > NOW() - INTERVAL '1 hour'
        "#,
    )
    .bind(ip)
    .bind(endpoint)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    count < MAX_LOGIN_ATTEMPTS
}

async fn record_attempt(pool: &PgPool, ip: &str, endpoint: &str) {
    let _ = sqlx::query(
        "INSERT INTO rate_limit_attempts (ip_address, endpoint) VALUES ($1, $2)",
    )
    .bind(ip)
    .bind(endpoint)
    .execute(pool)
    .await;
}

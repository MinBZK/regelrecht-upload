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
use sqlx::PgPool;
use uuid::Uuid;

use super::AppState;

/// Session cookie name
pub const SESSION_COOKIE: &str = "rr_admin_session";

/// Rate limit: max attempts per IP per hour
const MAX_LOGIN_ATTEMPTS: i64 = 10;

/// Rate limit: max submission creations per IP per hour
pub(crate) const MAX_SUBMISSION_ATTEMPTS: i64 = 20;

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
            tracing::error!(
                "Invalid password hash in database for user {}",
                user.username
            );
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
    let secure_flag = if state.is_production { "; Secure" } else { "" };
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        SESSION_COOKIE,
        token,
        8 * 3600, // 8 hours
        secure_flag
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::success(AdminUserResponse::from(user))),
    )
}

/// Admin logout
pub async fn admin_logout(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let token = extract_session_token(&headers);

    if let Some(token) = token {
        let token_hash = hash_token(&token);

        // Get session for audit log
        let session =
            sqlx::query_as::<_, AdminSession>("SELECT * FROM admin_sessions WHERE token_hash = $1")
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
    let secure_flag = if state.is_production { "; Secure" } else { "" };
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{}",
        SESSION_COOKIE, secure_flag
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
        Some(user) => (
            StatusCode::OK,
            Json(ApiResponse::success(AdminUserResponse::from(user))),
        ),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Not authenticated")),
        ),
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
    let session = match sqlx::query_as::<_, AdminSession>(
        r#"
        SELECT * FROM admin_sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::debug!("No valid session found for token hash");
            return None;
        }
        Err(e) => {
            tracing::error!("Database error during session lookup: {}", e);
            return None;
        }
    };

    // Get associated user
    match sqlx::query_as::<_, AdminUser>(
        "SELECT * FROM admin_users WHERE id = $1 AND is_active = true",
    )
    .bind(session.admin_user_id)
    .fetch_optional(pool)
    .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Database error fetching admin user: {}", e);
            None
        }
    }
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

/// Seed admin user from environment variables (ADMIN_USERNAME, ADMIN_EMAIL, ADMIN_PASSWORD)
pub async fn seed_admin_user(pool: &PgPool) {
    let username = match std::env::var("ADMIN_USERNAME") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let email = match std::env::var("ADMIN_EMAIL") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let password = match std::env::var("ADMIN_PASSWORD") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };

    // Check if user already exists
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM admin_users WHERE username = $1")
            .bind(&username)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    if existing.is_some() {
        tracing::info!("Admin user '{}' already exists, skipping seed", username);
        return;
    }

    match create_admin_user(pool, &username, &email, &password, Some(&username)).await {
        Ok(user) => {
            tracing::info!("Seeded admin user '{}' (id: {})", user.username, user.id);
        }
        Err(e) => {
            tracing::error!("Failed to seed admin user: {}", e);
        }
    }
}

/// Create an admin user (utility function for setup)
pub async fn create_admin_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    password: &str,
    display_name: Option<&str>,
) -> Result<AdminUser, sqlx::Error> {
    let password_hash =
        hash_password(password).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

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

pub(crate) fn extract_session_token(headers: &HeaderMap) -> Option<String> {
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
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub(crate) fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub(crate) fn get_client_ip(headers: &HeaderMap) -> String {
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

pub(crate) async fn check_rate_limit_with_max(
    pool: &PgPool,
    ip: &str,
    endpoint: &str,
    max_attempts: i64,
) -> bool {
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

    count < max_attempts
}

pub(crate) async fn check_rate_limit(pool: &PgPool, ip: &str, endpoint: &str) -> bool {
    check_rate_limit_with_max(pool, ip, endpoint, MAX_LOGIN_ATTEMPTS).await
}

pub(crate) async fn record_attempt(pool: &PgPool, ip: &str, endpoint: &str) {
    let _ = sqlx::query("INSERT INTO rate_limit_attempts (ip_address, endpoint) VALUES ($1, $2)")
        .bind(ip)
        .bind(endpoint)
        .execute(pool)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_token_is_sha256() {
        let hash = hash_token("test-token");
        // SHA-256 produces 64-character hex string
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hash_token_is_deterministic() {
        let hash1 = hash_token("same-token");
        let hash2 = hash_token("same-token");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_token_different_inputs() {
        let hash1 = hash_token("token-a");
        let hash2 = hash_token("token-b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_session_token_length() {
        let token = generate_session_token();
        // 32 random bytes = 64 hex chars
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_session_token_unique() {
        let t1 = generate_session_token();
        let t2 = generate_session_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_hash_password_and_verify() {
        let password = "test-password-123!";
        let hash = hash_password(password).unwrap();

        // Hash should be an Argon2 hash
        assert!(hash.starts_with("$argon2"));

        // Verify should succeed
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok());
    }

    #[test]
    fn test_hash_password_wrong_password() {
        let hash = hash_password("correct-password").unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default()
            .verify_password(b"wrong-password", &parsed)
            .is_err());
    }

    #[test]
    fn test_extract_session_token_from_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "rr_admin_session=abc123; other=xyz".parse().unwrap(),
        );
        assert_eq!(extract_session_token(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_session_token_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_session_token(&headers), None);
    }

    #[test]
    fn test_extract_session_token_wrong_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "other_cookie=abc123".parse().unwrap());
        assert_eq!(extract_session_token(&headers), None);
    }

    #[test]
    fn test_get_client_ip_xff() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "1.2.3.4");
    }

    #[test]
    fn test_get_client_ip_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
        assert_eq!(get_client_ip(&headers), "10.0.0.1");
    }

    #[test]
    fn test_get_client_ip_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(get_client_ip(&headers), "unknown");
    }
}

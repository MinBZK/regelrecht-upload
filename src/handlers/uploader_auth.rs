//! Uploader authentication handlers for self-service dossier access
//!
//! Allows uploaders to authenticate using their submission slug + email combination
//! to add documents to their dossier after initial submission.

use crate::models::*;
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::auth::{check_rate_limit, get_client_ip, record_attempt};
use super::AppState;

/// Session cookie name for uploader sessions
pub const UPLOADER_SESSION_COOKIE: &str = "rr_uploader_session";

/// Session duration in hours
const UPLOADER_SESSION_HOURS: i64 = 4;

// =============================================================================
// Login Endpoint
// =============================================================================

/// Uploader login - authenticate with slug + email
pub async fn uploader_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<UploaderLoginRequest>,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers, &state.trusted_proxies);

    // Check rate limit (10 attempts per hour per IP)
    if !check_rate_limit(&state.pool, &client_ip, "uploader_login").await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::<UploaderSessionResponse>::error(
                "Te veel inlogpogingen. Probeer het later opnieuw.",
            )),
        );
    }

    // Record attempt for rate limiting
    record_attempt(&state.pool, &client_ip, "uploader_login").await;

    // Validate input
    let slug = input.slug.trim().to_lowercase();
    let email = input.email.trim().to_lowercase();

    if slug.is_empty() || email.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::error(
                "Vul zowel referentiecode als e-mailadres in.",
            )),
        );
    }

    // Find submission by slug AND email (case-insensitive)
    let submission = sqlx::query_as::<_, Submission>(
        r#"
        SELECT * FROM submissions
        WHERE LOWER(slug) = $1
        AND LOWER(submitter_email) = $2
        "#,
    )
    .bind(&slug)
    .bind(&email)
    .fetch_optional(&state.pool)
    .await;

    let submission = match submission {
        Ok(Some(s)) => s,
        Ok(None) | Err(_) => {
            // Don't reveal whether slug or email was wrong
            return (
                StatusCode::UNAUTHORIZED,
                [(header::SET_COOKIE, "".to_string())],
                Json(ApiResponse::error(
                    "Ongeldige referentiecode of e-mailadres.",
                )),
            );
        }
    };

    // Check if submission has an email (required for this auth method)
    if submission.submitter_email.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::error(
                "Deze inzending heeft geen e-mailadres gekoppeld.",
            )),
        );
    }

    // Generate session token
    let token = generate_session_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(UPLOADER_SESSION_HOURS);

    // Get user agent for audit
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(500).collect::<String>());

    // Create session
    let session_result = sqlx::query(
        r#"
        INSERT INTO uploader_sessions (submission_id, email, token_hash, expires_at, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(submission.id)
    .bind(&email)
    .bind(&token_hash)
    .bind(expires_at)
    .bind(&client_ip)
    .bind(&user_agent)
    .execute(&state.pool)
    .await;

    if session_result.is_err() {
        tracing::error!("Failed to create uploader session: {:?}", session_result);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::SET_COOKIE, "".to_string())],
            Json(ApiResponse::error("Kon sessie niet aanmaken.")),
        );
    }

    // Log audit event
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_ip)
        VALUES ('uploader_login'::audit_action, 'submission', $1, 'uploader', $2)
        "#,
    )
    .bind(submission.id)
    .bind(&client_ip)
    .execute(&state.pool)
    .await;

    // Get documents for response
    let documents = sqlx::query_as::<_, Document>(
        "SELECT * FROM documents WHERE submission_id = $1 ORDER BY created_at",
    )
    .bind(submission.id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Build response (privacy-focused: no name/org)
    let response = UploaderSessionResponse {
        submission_id: submission.id,
        slug: submission.slug,
        status: submission.status,
        documents: documents.into_iter().map(DocumentResponse::from).collect(),
        session_expires_at: expires_at,
    };

    // Set secure cookie
    let secure_flag = if state.is_production { "; Secure" } else { "" };
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        UPLOADER_SESSION_COOKIE,
        token,
        UPLOADER_SESSION_HOURS * 3600,
        secure_flag
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::success(response)),
    )
}

// =============================================================================
// Logout Endpoint
// =============================================================================

/// Uploader logout - end session
pub async fn uploader_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = extract_uploader_session_token(&headers);
    let client_ip = get_client_ip(&headers, &state.trusted_proxies);

    if let Some(token) = token {
        let token_hash = hash_token(&token);

        // Get session for audit log
        let session = sqlx::query_as::<_, UploaderSession>(
            "SELECT * FROM uploader_sessions WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        // Delete session
        let _ = sqlx::query("DELETE FROM uploader_sessions WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(&state.pool)
            .await;

        // Log audit event
        if let Some(session) = session {
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_ip)
                VALUES ('uploader_logout'::audit_action, 'submission', $1, 'uploader', $2)
                "#,
            )
            .bind(session.submission_id)
            .bind(&client_ip)
            .execute(&state.pool)
            .await;
        }
    }

    // Clear cookie
    let secure_flag = if state.is_production { "; Secure" } else { "" };
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{}",
        UPLOADER_SESSION_COOKIE, secure_flag
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::success(())),
    )
}

// =============================================================================
// Get Current Uploader Session
// =============================================================================

/// Get current uploader session info
pub async fn get_current_uploader(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match validate_uploader_session(&state.pool, &headers).await {
        Some((submission, session)) => {
            // Get documents
            let documents = sqlx::query_as::<_, Document>(
                "SELECT * FROM documents WHERE submission_id = $1 ORDER BY created_at",
            )
            .bind(submission.id)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let response = UploaderSessionResponse {
                submission_id: submission.id,
                slug: submission.slug,
                status: submission.status,
                documents: documents.into_iter().map(DocumentResponse::from).collect(),
                session_expires_at: session.expires_at,
            };

            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Niet ingelogd.")),
        ),
    }
}

// =============================================================================
// Session Validation
// =============================================================================

/// Validate uploader session from headers and return the associated submission
pub async fn validate_uploader_session(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Option<(Submission, UploaderSession)> {
    let token = extract_uploader_session_token(headers)?;
    let token_hash = hash_token(&token);

    // Find valid session
    let session = match sqlx::query_as::<_, UploaderSession>(
        r#"
        SELECT * FROM uploader_sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::debug!("No valid uploader session found for token hash");
            return None;
        }
        Err(e) => {
            tracing::error!("Database error during uploader session lookup: {}", e);
            return None;
        }
    };

    // Get associated submission
    match sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE id = $1")
        .bind(session.submission_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(submission)) => Some((submission, session)),
        Ok(None) => {
            tracing::warn!(
                "Uploader session references non-existent submission: {}",
                session.submission_id
            );
            None
        }
        Err(e) => {
            tracing::error!("Database error fetching submission: {}", e);
            None
        }
    }
}

/// Check if an uploader session is valid for a specific submission
pub async fn validate_uploader_session_for_submission(
    pool: &PgPool,
    headers: &HeaderMap,
    submission_id: Uuid,
) -> bool {
    match validate_uploader_session(pool, headers).await {
        Some((submission, _)) => submission.id == submission_id,
        None => false,
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn extract_uploader_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", UPLOADER_SESSION_COOKIE)) {
            return Some(value.to_string());
        }
    }

    None
}

fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_token_is_sha256() {
        let hash = hash_token("test-uploader-token");
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
    fn test_extract_uploader_session_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "rr_uploader_session=abc123xyz; other=xyz".parse().unwrap(),
        );
        assert_eq!(
            extract_uploader_session_token(&headers),
            Some("abc123xyz".to_string())
        );
    }

    #[test]
    fn test_extract_uploader_session_token_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_uploader_session_token(&headers), None);
    }

    #[test]
    fn test_extract_uploader_session_token_wrong_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "rr_admin_session=abc123".parse().unwrap());
        assert_eq!(extract_uploader_session_token(&headers), None);
    }
}

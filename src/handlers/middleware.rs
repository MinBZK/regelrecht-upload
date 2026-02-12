//! Middleware for authentication and security headers

use crate::handlers::auth::{extract_session_token, hash_token};
use crate::handlers::AppState;
use crate::models::AdminUser;
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Admin user extracted by middleware, available via Extension<AdminUser>
pub async fn require_admin(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let headers = request.headers();
    let token = extract_session_token(headers);

    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({"success": false, "error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let token_hash = hash_token(&token);

    // Find valid session
    let session = sqlx::query_as::<_, crate::models::AdminSession>(
        "SELECT * FROM admin_sessions WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await;

    let session = match session {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({"success": false, "error": "Session expired or invalid"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error during session validation: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"success": false, "error": "Authentication error"})),
            )
                .into_response();
        }
    };

    // Get associated user
    let user = sqlx::query_as::<_, AdminUser>(
        "SELECT * FROM admin_users WHERE id = $1 AND is_active = true",
    )
    .bind(session.admin_user_id)
    .fetch_optional(&state.pool)
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({"success": false, "error": "User not found or inactive"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error fetching admin user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"success": false, "error": "Authentication error"})),
            )
                .into_response();
        }
    };

    // Insert AdminUser into request extensions
    let mut request = request;
    request.extensions_mut().insert(user);

    next.run(request).await
}

/// Security headers middleware
pub async fn security_headers(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; form-action 'self'; base-uri 'self'; frame-ancestors 'none'",
        ),
    );

    if state.is_production {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }

    response
}

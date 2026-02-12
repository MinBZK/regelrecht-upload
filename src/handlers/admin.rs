//! Admin portal handlers

use crate::handlers::auth::validate_admin_session;
use crate::models::*;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ListSubmissionsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<SubmissionStatus>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: SubmissionStatus,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForwardSubmissionRequest {
    pub forward_to: String,
    pub notes: Option<String>,
}

// =============================================================================
// Admin Submission Endpoints
// =============================================================================

/// List all submissions (admin)
pub async fn list_submissions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListSubmissionsQuery>,
) -> impl IntoResponse {
    // Validate admin session
    let admin = match validate_admin_session(&state.pool, &headers).await {
        Some(a) => a,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<PaginatedResponse<SubmissionResponse>>::error(
                    "Unauthorized",
                )),
            )
        }
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build query based on filters
    let (submissions, total): (Vec<Submission>, i64) = if let Some(status) = query.status {
        let subs = sqlx::query_as::<_, Submission>(
            r#"
            SELECT * FROM submissions
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM submissions WHERE status = $1")
            .bind(status)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

        (subs, count)
    } else if let Some(ref search) = query.search {
        let search_pattern = format!("%{}%", search);
        let subs = sqlx::query_as::<_, Submission>(
            r#"
            SELECT * FROM submissions
            WHERE submitter_name ILIKE $1
               OR organization ILIKE $1
               OR slug ILIKE $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&search_pattern)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM submissions
            WHERE submitter_name ILIKE $1
               OR organization ILIKE $1
               OR slug ILIKE $1
            "#,
        )
        .bind(&search_pattern)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

        (subs, count)
    } else {
        let subs = sqlx::query_as::<_, Submission>(
            "SELECT * FROM submissions ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM submissions")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

        (subs, count)
    };

    // Fetch documents for each submission
    let mut responses = Vec::new();
    for sub in submissions {
        let documents = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE submission_id = $1 ORDER BY created_at",
        )
        .bind(sub.id)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();

        responses.push(SubmissionResponse {
            id: sub.id,
            slug: sub.slug,
            submitter_name: sub.submitter_name,
            submitter_email: sub.submitter_email,
            organization: sub.organization,
            organization_department: sub.organization_department,
            status: sub.status,
            notes: sub.notes,
            created_at: sub.created_at,
            updated_at: sub.updated_at,
            submitted_at: sub.submitted_at,
            documents: documents.into_iter().map(DocumentResponse::from).collect(),
        });
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    tracing::info!(
        "Admin {} listed submissions (page {}, {} results)",
        admin.username,
        page,
        responses.len()
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(PaginatedResponse {
            items: responses,
            total,
            page,
            per_page,
            total_pages,
        })),
    )
}

/// Get submission details (admin)
pub async fn get_submission_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Validate admin session
    if validate_admin_session(&state.pool, &headers)
        .await
        .is_none()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<SubmissionResponse>::error("Unauthorized")),
        );
    }

    let submission = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await;

    match submission {
        Ok(Some(sub)) => {
            let documents = sqlx::query_as::<_, Document>(
                "SELECT * FROM documents WHERE submission_id = $1 ORDER BY created_at",
            )
            .bind(sub.id)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let response = SubmissionResponse {
                id: sub.id,
                slug: sub.slug,
                submitter_name: sub.submitter_name,
                submitter_email: sub.submitter_email,
                organization: sub.organization,
                organization_department: sub.organization_department,
                status: sub.status,
                notes: sub.notes,
                created_at: sub.created_at,
                updated_at: sub.updated_at,
                submitted_at: sub.submitted_at,
                documents: documents.into_iter().map(DocumentResponse::from).collect(),
            };

            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Submission not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Database error")),
            )
        }
    }
}

/// Update submission status (admin)
pub async fn update_submission_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    // Validate admin session
    let admin = match validate_admin_session(&state.pool, &headers).await {
        Some(a) => a,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<Submission>::error("Unauthorized")),
            )
        }
    };

    let result = sqlx::query_as::<_, Submission>(
        r#"
        UPDATE submissions
        SET status = $1, notes = COALESCE($2, notes)
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(input.status)
    .bind(&input.notes)
    .bind(id)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(submission)) => {
            // Log audit event
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id, details)
                VALUES ('submission_status_changed'::audit_action, 'submission', $1, 'admin', $2, $3)
                "#,
            )
            .bind(id)
            .bind(admin.id)
            .bind(serde_json::json!({
                "new_status": input.status,
                "notes": input.notes
            }))
            .execute(&state.pool)
            .await;

            tracing::info!(
                "Admin {} changed submission {} status to {:?}",
                admin.username,
                id,
                input.status
            );

            (StatusCode::OK, Json(ApiResponse::success(submission)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Submission not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to update status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to update status")),
            )
        }
    }
}

/// Forward submission to RegelRecht team (admin)
pub async fn forward_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<ForwardSubmissionRequest>,
) -> impl IntoResponse {
    // Validate admin session
    let admin = match validate_admin_session(&state.pool, &headers).await {
        Some(a) => a,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<Submission>::error("Unauthorized")),
            )
        }
    };

    // Update status to forwarded
    let result = sqlx::query_as::<_, Submission>(
        r#"
        UPDATE submissions
        SET status = 'forwarded', notes = COALESCE($1, notes)
        WHERE id = $2 AND status IN ('submitted', 'under_review', 'approved')
        RETURNING *
        "#,
    )
    .bind(&input.notes)
    .bind(id)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(submission)) => {
            // Log audit event with forward details
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id, details)
                VALUES ('submission_status_changed'::audit_action, 'submission', $1, 'admin', $2, $3)
                "#,
            )
            .bind(id)
            .bind(admin.id)
            .bind(serde_json::json!({
                "action": "forwarded",
                "forward_to": input.forward_to,
                "notes": input.notes
            }))
            .execute(&state.pool)
            .await;

            tracing::info!(
                "Admin {} forwarded submission {} to {}",
                admin.username,
                id,
                input.forward_to
            );

            (StatusCode::OK, Json(ApiResponse::success(submission)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "Submission not found or not in a forwardable status",
            )),
        ),
        Err(e) => {
            tracing::error!("Failed to forward submission: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to forward submission")),
            )
        }
    }
}

/// Get admin dashboard statistics
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Validate admin session
    if validate_admin_session(&state.pool, &headers)
        .await
        .is_none()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error("Unauthorized")),
        );
    }

    // Get counts by status
    let stats = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT status::text, COUNT(*) as count
        FROM submissions
        GROUP BY status
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let total_documents: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let pending_slots: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM calendar_slots WHERE is_available = true AND slot_start > NOW()",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let stats_map: std::collections::HashMap<String, i64> = stats.into_iter().collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "submissions_by_status": stats_map,
            "total_documents": total_documents,
            "available_meeting_slots": pending_slots
        }))),
    )
}

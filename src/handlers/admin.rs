//! Admin portal handlers

use crate::models::*;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use uuid::Uuid;
use zip::write::FileOptions;
use zip::ZipWriter;

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
    Extension(admin): Extension<AdminUser>,
    Query(query): Query<ListSubmissionsQuery>,
) -> impl IntoResponse {
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

    // Batch fetch documents for all submissions (avoid N+1 query)
    let submission_ids: Vec<Uuid> = submissions.iter().map(|s| s.id).collect();
    let all_documents = if submission_ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE submission_id = ANY($1) ORDER BY created_at",
        )
        .bind(&submission_ids)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    };

    // Group documents by submission_id
    let mut docs_by_submission: std::collections::HashMap<Uuid, Vec<Document>> =
        std::collections::HashMap::new();
    for doc in all_documents {
        docs_by_submission
            .entry(doc.submission_id)
            .or_default()
            .push(doc);
    }

    let mut responses = Vec::new();
    for sub in submissions {
        let documents = docs_by_submission.remove(&sub.id).unwrap_or_default();

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
            retention_expiry_date: sub.retention_expiry_date,
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
    Extension(_admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
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
                retention_expiry_date: sub.retention_expiry_date,
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
    Extension(admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
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
    Extension(admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<ForwardSubmissionRequest>,
) -> impl IntoResponse {
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

/// Delete a submission (admin)
pub async fn delete_submission(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // 1. Fetch the submission to get the slug for file cleanup
    let submission = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await;

    match submission {
        Ok(Some(sub)) => {
            // 2. Delete files from disk before database cascade
            let submission_dir = state.upload_dir.join(&sub.slug);
            if submission_dir.exists() {
                if let Err(e) = tokio::fs::remove_dir_all(&submission_dir).await {
                    tracing::warn!(
                        "Failed to remove submission directory {:?}: {}",
                        submission_dir,
                        e
                    );
                    // Continue with database deletion even if file cleanup fails
                }
            }

            // 3. Delete from database (CASCADE handles documents + uploader_sessions)
            let delete_result = sqlx::query("DELETE FROM submissions WHERE id = $1")
                .bind(id)
                .execute(&state.pool)
                .await;

            match delete_result {
                Ok(_) => {
                    // 4. Log audit event
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id, details)
                        VALUES ('data_deleted'::audit_action, 'submission', $1, 'admin', $2, $3)
                        "#,
                    )
                    .bind(id)
                    .bind(admin.id)
                    .bind(serde_json::json!({
                        "slug": sub.slug,
                        "submitter_name": sub.submitter_name,
                        "organization": sub.organization,
                        "deleted_by": admin.username
                    }))
                    .execute(&state.pool)
                    .await;

                    tracing::info!(
                        "Admin {} deleted submission {} ({})",
                        admin.username,
                        id,
                        sub.slug
                    );

                    (
                        StatusCode::OK,
                        Json(ApiResponse::success(serde_json::json!({
                            "deleted": true,
                            "id": id,
                            "slug": sub.slug
                        }))),
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to delete submission: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to delete submission")),
                    )
                }
            }
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

/// Get admin dashboard statistics
pub async fn get_dashboard_stats(
    State(state): State<AppState>,
    Extension(_admin): Extension<AdminUser>,
) -> impl IntoResponse {
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

// =============================================================================
// Export Endpoints
// =============================================================================

/// Export submission data as JSON
#[derive(Debug, Serialize)]
pub struct SubmissionExport {
    pub submission: SubmissionResponse,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub exported_by: String,
}

pub async fn export_submission_json(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
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
                slug: sub.slug.clone(),
                submitter_name: sub.submitter_name,
                submitter_email: sub.submitter_email,
                organization: sub.organization,
                organization_department: sub.organization_department,
                status: sub.status,
                notes: sub.notes,
                created_at: sub.created_at,
                updated_at: sub.updated_at,
                submitted_at: sub.submitted_at,
                retention_expiry_date: sub.retention_expiry_date,
                documents: documents.into_iter().map(DocumentResponse::from).collect(),
            };

            let export = SubmissionExport {
                submission: response,
                exported_at: chrono::Utc::now(),
                exported_by: admin.username.clone(),
            };

            tracing::info!(
                "Admin {} exported submission {} as JSON",
                admin.username,
                id
            );

            let json_data = serde_json::to_string_pretty(&export).unwrap_or_default();
            let filename = format!("submission_{}.json", sub.slug);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(json_data))
                .unwrap()
        }
        Ok(None) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_string(&ApiResponse::<()>::error("Submission not found")).unwrap(),
            ))
            .unwrap(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&ApiResponse::<()>::error("Database error")).unwrap(),
                ))
                .unwrap()
        }
    }
}

/// Export submission files as ZIP
pub async fn export_submission_files(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminUser>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
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

            // Create ZIP file in memory
            let mut zip_buffer = Cursor::new(Vec::new());
            {
                let mut zip = ZipWriter::new(&mut zip_buffer);
                let options =
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

                // Add submission metadata as JSON
                let metadata = SubmissionExport {
                    submission: SubmissionResponse {
                        id: sub.id,
                        slug: sub.slug.clone(),
                        submitter_name: sub.submitter_name.clone(),
                        submitter_email: sub.submitter_email.clone(),
                        organization: sub.organization.clone(),
                        organization_department: sub.organization_department.clone(),
                        status: sub.status,
                        notes: sub.notes.clone(),
                        created_at: sub.created_at,
                        updated_at: sub.updated_at,
                        submitted_at: sub.submitted_at,
                        retention_expiry_date: sub.retention_expiry_date,
                        documents: documents
                            .iter()
                            .cloned()
                            .map(DocumentResponse::from)
                            .collect(),
                    },
                    exported_at: chrono::Utc::now(),
                    exported_by: admin.username.clone(),
                };

                let metadata_json = serde_json::to_string_pretty(&metadata).unwrap_or_default();
                if zip.start_file("metadata.json", options).is_ok() {
                    let _ = zip.write_all(metadata_json.as_bytes());
                }

                // Add each document file
                for doc in &documents {
                    if let Some(ref file_path) = doc.file_path {
                        let path = std::path::Path::new(file_path);
                        if path.exists() {
                            if let Ok(file_data) = tokio::fs::read(path).await {
                                let fallback = doc
                                    .filename
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string());
                                let filename = doc.original_filename.as_ref().unwrap_or(&fallback);
                                if zip
                                    .start_file(format!("files/{}", filename), options)
                                    .is_ok()
                                {
                                    let _ = zip.write_all(&file_data);
                                }
                            }
                        }
                    }
                }

                let _ = zip.finish();
            }

            tracing::info!(
                "Admin {} exported submission {} files as ZIP",
                admin.username,
                id
            );

            let zip_data = zip_buffer.into_inner();
            let filename = format!("submission_{}_files.zip", sub.slug);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/zip")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(zip_data))
                .unwrap()
        }
        Ok(None) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::to_string(&ApiResponse::<()>::error("Submission not found")).unwrap(),
            ))
            .unwrap(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_string(&ApiResponse::<()>::error("Database error")).unwrap(),
                ))
                .unwrap()
        }
    }
}

// =============================================================================
// Maintenance Functions
// =============================================================================

/// Clean up abandoned draft submissions older than 1 hour
///
/// This function is called periodically from the cleanup task in main.rs.
/// It removes draft submissions that were never submitted, including their
/// files from disk.
pub async fn cleanup_abandoned_drafts(
    pool: &sqlx::PgPool,
    upload_dir: &std::path::Path,
) -> Result<u64, sqlx::Error> {
    // 1. Find and delete drafts older than 1 hour, returning the deleted rows
    //    This is atomic - no race condition between finding and deleting
    let deleted_drafts = sqlx::query_as::<_, Submission>(
        r#"
        DELETE FROM submissions
        WHERE status = 'draft'
        AND created_at < NOW() - INTERVAL '1 hour'
        RETURNING *
        "#,
    )
    .fetch_all(pool)
    .await?;

    if deleted_drafts.is_empty() {
        return Ok(0);
    }

    let count = deleted_drafts.len();

    // 2. Delete files from disk for each deleted draft
    //    Safe because these drafts are already deleted from DB
    for draft in &deleted_drafts {
        let draft_dir = upload_dir.join(&draft.slug);
        if draft_dir.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&draft_dir).await {
                tracing::warn!(
                    "Failed to remove abandoned draft directory {:?}: {}",
                    draft_dir,
                    e
                );
            }
        }
    }

    tracing::info!("Cleaned up {} abandoned draft submissions", count);

    Ok(count as u64)
}

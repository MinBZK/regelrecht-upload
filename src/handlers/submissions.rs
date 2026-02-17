//! Submission handlers for the applicant portal

use crate::handlers::auth::{
    check_rate_limit_with_max, get_client_ip, record_attempt, MAX_SUBMISSION_ATTEMPTS,
};
use crate::handlers::uploader_auth::validate_uploader_session;
use crate::models::*;
use crate::validation::{
    validate_classification_for_upload, validate_create_submission, validate_external_url,
    validate_file_upload, validate_filename_extensions, validate_slug,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub upload_dir: PathBuf,
    pub max_upload_size: usize,
    pub is_production: bool,
    /// Trusted proxy IP prefixes for X-Forwarded-For validation
    pub trusted_proxies: Vec<String>,
}

// =============================================================================
// Submission Endpoints
// =============================================================================

/// Create a new submission
pub async fn create_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateSubmission>,
) -> impl IntoResponse {
    // Rate limit submission creation
    let client_ip = get_client_ip(&headers, &state.trusted_proxies);
    if !check_rate_limit_with_max(
        &state.pool,
        &client_ip,
        "create_submission",
        MAX_SUBMISSION_ATTEMPTS,
    )
    .await
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResponse::<Submission>::error(
                "Too many submissions. Please try again later.",
            )),
        );
    }
    record_attempt(&state.pool, &client_ip, "create_submission").await;

    // Validate input
    if let Err(e) = validate_create_submission(&input) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<Submission>::error(e.to_string())),
        );
    }

    // Generate slug
    let slug: String = sqlx::query_scalar("SELECT generate_submission_slug()")
        .fetch_one(&state.pool)
        .await
        .unwrap_or_else(|_| {
            format!(
                "rr-{}-{}",
                chrono::Utc::now().format("%Y%m%d"),
                &Uuid::new_v4().to_string()[..5]
            )
        });

    // Insert submission
    let result = sqlx::query_as::<_, Submission>(
        r#"
        INSERT INTO submissions (slug, submitter_name, submitter_email, organization, organization_department)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&slug)
    .bind(&input.submitter_name)
    .bind(&input.submitter_email)
    .bind(&input.organization)
    .bind(&input.organization_department)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(submission) => {
            // Log audit event
            log_audit(
                &state.pool,
                "submission_created",
                "submission",
                Some(submission.id),
                "applicant",
                None,
            )
            .await;

            (StatusCode::CREATED, Json(ApiResponse::success(submission)))
        }
        Err(e) => {
            tracing::error!("Failed to create submission: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to create submission")),
            )
        }
    }
}

/// Get submission by slug
pub async fn get_submission(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<SubmissionResponse>::error(e.to_string())),
        );
    }

    // Get submission
    let submission = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await;

    match submission {
        Ok(Some(submission)) => {
            // Get documents
            let documents = sqlx::query_as::<_, Document>(
                "SELECT * FROM documents WHERE submission_id = $1 ORDER BY created_at",
            )
            .bind(submission.id)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();

            let response = SubmissionResponse {
                id: submission.id,
                slug: submission.slug,
                submitter_name: submission.submitter_name,
                submitter_email: submission.submitter_email,
                organization: submission.organization,
                organization_department: submission.organization_department,
                status: submission.status,
                notes: submission.notes,
                created_at: submission.created_at,
                updated_at: submission.updated_at,
                submitted_at: submission.submitted_at,
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

/// Update submission
pub async fn update_submission(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(input): Json<UpdateSubmission>,
) -> impl IntoResponse {
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<Submission>::error(e.to_string())),
        );
    }

    // Check submission exists and is in draft status
    let existing = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await;

    match existing {
        Ok(Some(submission)) => {
            if submission.status != SubmissionStatus::Draft {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "Cannot update submission that is not in draft status",
                    )),
                );
            }

            // Build dynamic update query
            let result = sqlx::query_as::<_, Submission>(
                r#"
                UPDATE submissions SET
                    submitter_name = COALESCE($1, submitter_name),
                    submitter_email = COALESCE($2, submitter_email),
                    organization = COALESCE($3, organization),
                    organization_department = COALESCE($4, organization_department),
                    notes = COALESCE($5, notes)
                WHERE slug = $6
                RETURNING *
                "#,
            )
            .bind(&input.submitter_name)
            .bind(&input.submitter_email)
            .bind(&input.organization)
            .bind(&input.organization_department)
            .bind(&input.notes)
            .bind(&slug)
            .fetch_one(&state.pool)
            .await;

            match result {
                Ok(updated) => {
                    log_audit(
                        &state.pool,
                        "submission_updated",
                        "submission",
                        Some(updated.id),
                        "applicant",
                        None,
                    )
                    .await;
                    (StatusCode::OK, Json(ApiResponse::success(updated)))
                }
                Err(e) => {
                    tracing::error!("Failed to update submission: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to update submission")),
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

/// Submit a submission (change status from draft to submitted)
pub async fn submit_submission(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<Submission>::error(e.to_string())),
        );
    }

    let result = sqlx::query_as::<_, Submission>(
        r#"
        UPDATE submissions
        SET status = 'submitted', submitted_at = NOW()
        WHERE slug = $1 AND status = 'draft'
        RETURNING *
        "#,
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(submission)) => {
            log_audit(
                &state.pool,
                "submission_submitted",
                "submission",
                Some(submission.id),
                "applicant",
                None,
            )
            .await;
            (StatusCode::OK, Json(ApiResponse::success(submission)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "Submission not found or not in draft status",
            )),
        ),
        Err(e) => {
            tracing::error!("Failed to submit: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to submit")),
            )
        }
    }
}

// =============================================================================
// Document Endpoints
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct UploadDocumentQuery {
    pub category: DocumentCategory,
    pub classification: DocumentClassification,
    pub description: Option<String>,
}

/// Upload a document
pub async fn upload_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Query(query): Query<UploadDocumentQuery>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    tracing::info!(
        "Upload request received for slug={}, category={:?}, classification={:?}",
        slug,
        query.category,
        query.classification
    );

    // Validate slug
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<DocumentResponse>::error(e.to_string())),
        );
    }

    // Check classification - reject restricted documents
    if let Err(e) = validate_classification_for_upload(query.classification) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!(
                "{}. Documents marked as 'restricted' cannot be uploaded to this portal. \
                Please only upload documents that may be used with AI tools.",
                e
            ))),
        );
    }

    // For formal laws, reject file uploads
    if query.category == DocumentCategory::FormalLaw {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "Formal laws should be added as links, not file uploads. \
                Use the /api/submissions/{slug}/formal-law endpoint instead.",
            )),
        );
    }

    // Get submission
    let submission = match get_submission_by_slug(&state.pool, &slug).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Submission not found")),
            )
        }
    };

    // Authorization check:
    // - Draft submissions: anyone with the slug can upload (existing behavior)
    // - Non-draft submissions: require valid uploader session for this specific submission
    if submission.status != SubmissionStatus::Draft {
        match validate_uploader_session(&state.pool, &headers).await {
            Some((session_submission, _)) if session_submission.id == submission.id => {
                // Valid session for this submission - allow upload
            }
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error(
                        "Inloggen vereist om documenten toe te voegen aan een ingediende inzending.",
                    )),
                );
            }
        }
    }

    // Process multipart upload (single file) with proper error handling
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("No file provided")),
            );
        }
        Err(e) => {
            tracing::error!("Multipart parsing error: {}", e);
            // Provide user-friendly error messages for common issues
            let error_msg = if e.to_string().contains("length limit") {
                "File too large. Maximum upload size is 50MB."
            } else if e.to_string().contains("content-type") {
                "Invalid upload format. Please use multipart/form-data."
            } else {
                "Failed to process upload. Please try again."
            };
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!("{} ({})", error_msg, e))),
            );
        }
    };

    let original_filename = field.file_name().unwrap_or("unknown").to_string();
    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to read file bytes: {}", e);
            let error_msg = if e.to_string().contains("length limit") {
                "File too large. Maximum upload size is 50MB."
            } else if e.to_string().contains("connection") {
                "Connection interrupted during upload. Please try again."
            } else {
                "Failed to read uploaded file. Please try again."
            };
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!("{} ({})", error_msg, e))),
            );
        }
    };

    // Validate file
    if let Err(e) = validate_file_upload(&content_type, data.len(), state.max_upload_size) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(e.to_string())),
        );
    }

    // Validate filename doesn't contain dangerous extensions
    if let Err(e) = validate_filename_extensions(&original_filename) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(e.to_string())),
        );
    }

    // Create storage path
    let doc_id = Uuid::new_v4();
    let safe_filename = sanitize_filename(&original_filename);
    let storage_filename = format!("{}_{}", doc_id, safe_filename);
    let submission_dir = state.upload_dir.join(&slug);

    // Create directory with detailed error logging
    if let Err(e) = fs::create_dir_all(&submission_dir).await {
        tracing::error!(
            "Failed to create upload directory {:?}: {} (kind: {:?})",
            submission_dir,
            e,
            e.kind()
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!(
                "Failed to create storage directory: {} ({:?})",
                e,
                e.kind()
            ))),
        );
    }

    // Write file - verify path stays within upload directory
    let file_path = submission_dir.join(&storage_filename);
    if !file_path.starts_with(&state.upload_dir) {
        tracing::error!(
            "Path traversal attempt detected: {:?} escapes {:?}",
            file_path,
            state.upload_dir
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid filename")),
        );
    }

    if let Err(e) = fs::write(&file_path, &data).await {
        tracing::error!(
            "Failed to write file {:?}: {} (kind: {:?})",
            file_path,
            e,
            e.kind()
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!(
                "Failed to write file: {} ({:?})",
                e,
                e.kind()
            ))),
        );
    }

    // Store metadata in database
    let result = sqlx::query_as::<_, Document>(
        r#"
        INSERT INTO documents (
            id, submission_id, category, classification,
            filename, original_filename, file_path, file_size, mime_type, description
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(doc_id)
    .bind(submission.id)
    .bind(query.category)
    .bind(query.classification)
    .bind(&storage_filename)
    .bind(&original_filename)
    .bind(file_path.to_string_lossy().to_string())
    .bind(data.len() as i64)
    .bind(&content_type)
    .bind(&query.description)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(doc) => {
            log_audit(
                &state.pool,
                "document_uploaded",
                "document",
                Some(doc.id),
                "applicant",
                None,
            )
            .await;
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(DocumentResponse::from(doc))),
            )
        }
        Err(e) => {
            tracing::error!("Failed to store document metadata: {}", e);
            // Clean up file - log if cleanup fails
            if let Err(cleanup_err) = fs::remove_file(&file_path).await {
                tracing::warn!(
                    "Failed to clean up orphaned file {:?}: {}",
                    file_path,
                    cleanup_err
                );
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "Failed to store document. Please try again.",
                )),
            )
        }
    }
}

/// Add a formal law link
pub async fn add_formal_law(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(input): Json<CreateFormalLaw>,
) -> impl IntoResponse {
    // Validate slug
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<DocumentResponse>::error(e.to_string())),
        );
    }

    // Validate URL
    if let Err(e) = validate_external_url(&input.external_url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(e.to_string())),
        );
    }

    // Get submission
    let submission = match get_submission_by_slug(&state.pool, &slug).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Submission not found")),
            )
        }
    };

    // Authorization check:
    // - Draft submissions: anyone with the slug can add laws (existing behavior)
    // - Non-draft submissions: require valid uploader session for this specific submission
    if submission.status != SubmissionStatus::Draft {
        match validate_uploader_session(&state.pool, &headers).await {
            Some((session_submission, _)) if session_submission.id == submission.id => {
                // Valid session for this submission - allow adding law
            }
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error(
                        "Inloggen vereist om documenten toe te voegen aan een ingediende inzending.",
                    )),
                );
            }
        }
    }

    // Formal laws are always public
    let result = sqlx::query_as::<_, Document>(
        r#"
        INSERT INTO documents (
            submission_id, category, classification,
            external_url, external_title, description
        )
        VALUES ($1, 'formal_law', 'public', $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(submission.id)
    .bind(&input.external_url)
    .bind(&input.external_title)
    .bind(&input.description)
    .fetch_one(&state.pool)
    .await;

    match result {
        Ok(doc) => {
            log_audit(
                &state.pool,
                "document_uploaded",
                "document",
                Some(doc.id),
                "applicant",
                None,
            )
            .await;
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(DocumentResponse::from(doc))),
            )
        }
        Err(e) => {
            tracing::error!("Failed to add formal law: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to add formal law")),
            )
        }
    }
}

/// Delete a document
pub async fn delete_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((slug, doc_id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(e.to_string())),
        );
    }

    // Get submission and verify ownership
    let submission = match get_submission_by_slug(&state.pool, &slug).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Submission not found")),
            )
        }
    };

    // Authorization check:
    // - Draft submissions: anyone with the slug can delete (existing behavior)
    // - Non-draft submissions: require valid uploader session for this specific submission
    if submission.status != SubmissionStatus::Draft {
        match validate_uploader_session(&state.pool, &headers).await {
            Some((session_submission, _)) if session_submission.id == submission.id => {
                // Valid session for this submission - allow deletion
            }
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error(
                        "Inloggen vereist om documenten te verwijderen van een ingediende inzending.",
                    )),
                );
            }
        }
    }

    // Get document
    let doc = sqlx::query_as::<_, Document>(
        "SELECT * FROM documents WHERE id = $1 AND submission_id = $2",
    )
    .bind(doc_id)
    .bind(submission.id)
    .fetch_optional(&state.pool)
    .await;

    match doc {
        Ok(Some(doc)) => {
            // Delete file if exists
            if let Some(ref file_path) = doc.file_path {
                let _ = fs::remove_file(file_path).await;
            }

            // Delete from database
            let _ = sqlx::query("DELETE FROM documents WHERE id = $1")
                .bind(doc_id)
                .execute(&state.pool)
                .await;

            log_audit(
                &state.pool,
                "document_deleted",
                "document",
                Some(doc_id),
                "applicant",
                None,
            )
            .await;

            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Document not found")),
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

// =============================================================================
// FAQ Endpoint
// =============================================================================

/// Get FAQ content
pub async fn get_faq() -> impl IntoResponse {
    let faq_items = vec![
        FaqItem {
            question: "Levert RegelRecht kant en klare regelsets?".to_string(),
            answer: "Nee. We doen een beleidsverkenning en onderzoeksproject naar de inzetbaarheid \
                van deze nieuwe technologie voor het vertalen van beleid naar machine-leesbare regels."
                .to_string(),
        },
        FaqItem {
            question: "Ik heb nog geen interne werkprocessen en wilde eigenlijk alleen beginnen bij de hoogste liggende wet".to_string(),
            answer: "We hebben meer informatie nodig om met je project aan de slag te kunnen. \
                De onderzoeksfocus van RegelRecht ligt op dit moment op het vertalen van \
                uitvoeringsbeleid en werkinstructies naar machine-leesbare regels, waarbij de \
                formele wetgeving als basis dient."
                .to_string(),
        },
        FaqItem {
            question: "Wie is eigenaar van de regels?".to_string(),
            answer: "De casushouder (uploader) is eigenaar van de regels. De uitkomsten van de \
                verkenningen met RegelRecht kunnen een eerste stap zijn, maar RegelRecht is niet \
                verantwoordelijk voor de juridische sluitendheid van het proces."
                .to_string(),
        },
        FaqItem {
            question: "Wat gebeurt er verder met mijn uploads?".to_string(),
            answer: "We maken een eerste aanzet van regels. We willen graag aan de slag met \
                jullie experts. Daarvoor plannen we een meeting in. Het is nuttig als inhoudelijk \
                juridische experts aanschuiven om aanwijzingen te geven over ontwikkelrichtingen \
                en focuspunten in de regels. Vul na upload een tijdslot in om de output te bespreken."
                .to_string(),
        },
        FaqItem {
            question: "Welke documenten kan ik uploaden?".to_string(),
            answer: "U kunt circulaires, uitvoeringsbeleid en werkinstructies uploaden. \
                Voor formele wetten voegt u een link toe naar de versie op wetten.overheid.nl. \
                Let op: documenten die als 'restricted' geclassificeerd zijn kunnen niet worden \
                geüpload - alleen documenten die publiek gemaakt mogen worden of gebruikt mogen \
                worden met AI-tools."
                .to_string(),
        },
        FaqItem {
            question: "Hoe lang worden mijn gegevens bewaard?".to_string(),
            answer: "Uw gegevens worden bewaard tot 12 maanden na indiening. De exacte \
                vervaldatum is zichtbaar bij het opvragen van uw inzendingsstatus. Na afloop \
                worden de gegevens verwijderd, tenzij u toestemming geeft voor langer bewaren. \
                Zie onze privacyverklaring voor meer details."
                .to_string(),
        },
    ];

    Json(ApiResponse::success(faq_items))
}

// =============================================================================
// Helper Functions
// =============================================================================

async fn get_submission_by_slug(pool: &PgPool, slug: &str) -> Option<Submission> {
    sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

fn sanitize_filename(filename: &str) -> String {
    // Extract only the basename (strip any directory components)
    let basename = filename.rsplit(['/', '\\']).next().unwrap_or(filename);

    let sanitized: String = basename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == '.' {
                // Only allow a single dot for the file extension
                '.'
            } else {
                '_'
            }
        })
        .collect();

    // Remove leading dots (prevent hidden files / traversal like ..pdf)
    let sanitized = sanitized.trim_start_matches('.').trim_matches('_');

    if sanitized.is_empty() {
        "upload".to_string()
    } else {
        sanitized.to_string()
    }
}

async fn log_audit(
    pool: &PgPool,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    actor_type: &str,
    actor_id: Option<Uuid>,
) {
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id)
        VALUES ($1::audit_action, $2, $3, $4, $5)
        "#,
    )
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(actor_type)
    .bind(actor_id)
    .execute(pool)
    .await;
}

//! Calendar and meeting scheduling handlers

use crate::models::*;
use crate::validation::validate_slug;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;

// =============================================================================
// Query Parameters
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct AvailableSlotsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct BookSlotRequest {
    pub slot_id: Uuid,
}

// =============================================================================
// Public Calendar Endpoints
// =============================================================================

/// Get available meeting slots (public)
pub async fn get_available_slots(
    State(state): State<AppState>,
    Query(query): Query<AvailableSlotsQuery>,
) -> impl IntoResponse {
    let from = query.from.unwrap_or_else(Utc::now);
    let to = query
        .to
        .unwrap_or_else(|| from + chrono::Duration::days(30));

    let slots = sqlx::query_as::<_, CalendarSlot>(
        r#"
        SELECT * FROM calendar_slots
        WHERE is_available = true
          AND slot_start >= $1
          AND slot_start <= $2
        ORDER BY slot_start ASC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let responses: Vec<CalendarSlotResponse> =
        slots.into_iter().map(CalendarSlotResponse::from).collect();

    (StatusCode::OK, Json(ApiResponse::success(responses)))
}

/// Book a meeting slot for a submission
pub async fn book_slot(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(input): Json<BookSlotRequest>,
) -> impl IntoResponse {
    // Validate slug
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<CalendarSlotResponse>::error(e.to_string())),
        );
    }

    // Get submission
    let submission = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await;

    let submission = match submission {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Submission not found")),
            )
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Database error")),
            );
        }
    };

    // Check if submission already has a booked slot
    let existing_booking = sqlx::query_as::<_, CalendarSlot>(
        "SELECT * FROM calendar_slots WHERE booked_by_submission = $1",
    )
    .bind(submission.id)
    .fetch_optional(&state.pool)
    .await;

    if let Ok(Some(_)) = existing_booking {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "This submission already has a meeting booked",
            )),
        );
    }

    // Try to book the slot (atomic operation)
    let result = sqlx::query_as::<_, CalendarSlot>(
        r#"
        UPDATE calendar_slots
        SET is_available = false, booked_by_submission = $1
        WHERE id = $2 AND is_available = true AND slot_start > NOW()
        RETURNING *
        "#,
    )
    .bind(submission.id)
    .bind(input.slot_id)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(slot)) => {
            // Log audit event
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id, details)
                VALUES ('slot_booked'::audit_action, 'calendar_slot', $1, 'applicant', $2, $3)
                "#,
            )
            .bind(slot.id)
            .bind(submission.id)
            .bind(serde_json::json!({
                "submission_slug": slug,
                "slot_start": slot.slot_start,
                "slot_end": slot.slot_end
            }))
            .execute(&state.pool)
            .await;

            tracing::info!("Slot {} booked for submission {}", input.slot_id, slug);

            (
                StatusCode::OK,
                Json(ApiResponse::success(CalendarSlotResponse::from(slot))),
            )
        }
        Ok(None) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "Slot not available or has already been booked",
            )),
        ),
        Err(e) => {
            tracing::error!("Failed to book slot: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to book slot")),
            )
        }
    }
}

/// Cancel a booking
pub async fn cancel_booking(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    // Validate slug
    if let Err(e) = validate_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(e.to_string())),
        );
    }

    // Get submission
    let submission = sqlx::query_as::<_, Submission>("SELECT * FROM submissions WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.pool)
        .await;

    let submission = match submission {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("Submission not found")),
            )
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Database error")),
            );
        }
    };

    // Find and cancel booking
    let result = sqlx::query_as::<_, CalendarSlot>(
        r#"
        UPDATE calendar_slots
        SET is_available = true, booked_by_submission = NULL
        WHERE booked_by_submission = $1
        RETURNING *
        "#,
    )
    .bind(submission.id)
    .fetch_optional(&state.pool)
    .await;

    match result {
        Ok(Some(slot)) => {
            // Log audit event
            let _ = sqlx::query(
                r#"
                INSERT INTO audit_log (action, entity_type, entity_id, actor_type, actor_id)
                VALUES ('slot_cancelled'::audit_action, 'calendar_slot', $1, 'applicant', $2)
                "#,
            )
            .bind(slot.id)
            .bind(submission.id)
            .execute(&state.pool)
            .await;

            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("No booking found for this submission")),
        ),
        Err(e) => {
            tracing::error!("Failed to cancel booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to cancel booking")),
            )
        }
    }
}

// =============================================================================
// Admin Calendar Endpoints
// =============================================================================

/// List all slots (admin)
pub async fn list_slots_admin(
    State(state): State<AppState>,
    Extension(_admin): Extension<AdminUser>,
    Query(query): Query<AvailableSlotsQuery>,
) -> impl IntoResponse {
    let from = query
        .from
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    let to = query
        .to
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(60));

    let slots = sqlx::query_as::<_, CalendarSlot>(
        r#"
        SELECT * FROM calendar_slots
        WHERE slot_start >= $1 AND slot_start <= $2
        ORDER BY slot_start ASC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let responses: Vec<CalendarSlotResponse> =
        slots.into_iter().map(CalendarSlotResponse::from).collect();

    (StatusCode::OK, Json(ApiResponse::success(responses)))
}

/// Create new calendar slot(s) (admin)
pub async fn create_slots(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminUser>,
    Json(input): Json<Vec<CreateCalendarSlot>>,
) -> impl IntoResponse {
    let mut created_slots = Vec::new();

    for slot_input in input {
        // Validate time range
        if slot_input.slot_end <= slot_input.slot_start {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("End time must be after start time")),
            );
        }

        // Create slot
        let result = sqlx::query_as::<_, CalendarSlot>(
            r#"
            INSERT INTO calendar_slots (slot_start, slot_end, created_by, notes)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(slot_input.slot_start)
        .bind(slot_input.slot_end)
        .bind(admin.id)
        .bind(&slot_input.notes)
        .fetch_one(&state.pool)
        .await;

        match result {
            Ok(slot) => {
                created_slots.push(CalendarSlotResponse::from(slot));
            }
            Err(e) => {
                tracing::error!("Failed to create slot: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("Failed to create slot")),
                );
            }
        }
    }

    tracing::info!(
        "Admin {} created {} calendar slots",
        admin.username,
        created_slots.len()
    );

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(created_slots)),
    )
}

/// Delete a calendar slot (admin)
pub async fn delete_slot(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminUser>,
    Path(slot_id): Path<Uuid>,
) -> impl IntoResponse {
    // Check if slot is booked
    let slot = sqlx::query_as::<_, CalendarSlot>("SELECT * FROM calendar_slots WHERE id = $1")
        .bind(slot_id)
        .fetch_optional(&state.pool)
        .await;

    match slot {
        Ok(Some(slot)) => {
            if slot.booked_by_submission.is_some() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "Cannot delete a booked slot. Cancel the booking first.",
                    )),
                );
            }

            let _ = sqlx::query("DELETE FROM calendar_slots WHERE id = $1")
                .bind(slot_id)
                .execute(&state.pool)
                .await;

            tracing::info!("Admin {} deleted calendar slot {}", admin.username, slot_id);

            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Slot not found")),
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

//! Data models for the application

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// Enums
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "submission_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Forwarded,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "document_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentCategory {
    FormalLaw,
    Circular,
    ImplementationPolicy,
    WorkInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "document_classification", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentClassification {
    Public,
    ClaudeAllowed,
    Restricted,
}

// =============================================================================
// Submission
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Submission {
    pub id: Uuid,
    pub slug: String,
    pub submitter_name: String,
    pub submitter_email: Option<String>,
    pub organization: String,
    pub organization_department: Option<String>,
    pub status: SubmissionStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub retention_expiry_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmission {
    pub submitter_name: String,
    pub submitter_email: Option<String>,
    pub organization: String,
    pub organization_department: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubmission {
    pub submitter_name: Option<String>,
    pub submitter_email: Option<String>,
    pub organization: Option<String>,
    pub organization_department: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionResponse {
    pub id: Uuid,
    pub slug: String,
    pub submitter_name: String,
    pub submitter_email: Option<String>,
    pub organization: String,
    pub organization_department: Option<String>,
    pub status: SubmissionStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub retention_expiry_date: DateTime<Utc>,
    pub documents: Vec<DocumentResponse>,
}

// =============================================================================
// Document
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub category: DocumentCategory,
    pub classification: DocumentClassification,
    pub external_url: Option<String>,
    pub external_title: Option<String>,
    pub filename: Option<String>,
    pub original_filename: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFormalLaw {
    pub external_url: String,
    pub external_title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub category: DocumentCategory,
    pub classification: DocumentClassification,
    pub external_url: Option<String>,
    pub external_title: Option<String>,
    pub filename: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            category: doc.category,
            classification: doc.classification,
            external_url: doc.external_url,
            external_title: doc.external_title,
            filename: doc.original_filename,
            file_size: doc.file_size,
            mime_type: doc.mime_type,
            description: doc.description,
            created_at: doc.created_at,
        }
    }
}

// =============================================================================
// Admin User
// =============================================================================

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AdminUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<AdminUser> for AdminUserResponse {
    fn from(user: AdminUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            is_active: user.is_active,
            last_login_at: user.last_login_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// =============================================================================
// Admin Session
// =============================================================================

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AdminSession {
    pub id: Uuid,
    pub admin_user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

// =============================================================================
// Calendar
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CalendarSlot {
    pub id: Uuid,
    pub slot_start: DateTime<Utc>,
    pub slot_end: DateTime<Utc>,
    pub is_available: bool,
    pub booked_by_submission: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCalendarSlot {
    pub slot_start: DateTime<Utc>,
    pub slot_end: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalendarSlotResponse {
    pub id: Uuid,
    pub slot_start: DateTime<Utc>,
    pub slot_end: DateTime<Utc>,
    pub is_available: bool,
    pub booked_by_submission: Option<Uuid>,
    pub notes: Option<String>,
}

impl From<CalendarSlot> for CalendarSlotResponse {
    fn from(slot: CalendarSlot) -> Self {
        Self {
            id: slot.id,
            slot_start: slot.slot_start,
            slot_end: slot.slot_end,
            is_available: slot.is_available,
            booked_by_submission: slot.booked_by_submission,
            notes: slot.notes,
        }
    }
}

// =============================================================================
// API Responses
// =============================================================================

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// =============================================================================
// FAQ
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct FaqItem {
    pub question: String,
    pub answer: String,
}

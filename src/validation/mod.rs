//! Input validation module

use crate::models::{CreateSubmission, DocumentClassification};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Field '{field}' is required")]
    Required { field: String },

    #[error("Field '{field}' is too long (max {max} characters)")]
    TooLong { field: String, max: usize },

    #[error("Field '{field}' is too short (min {min} characters)")]
    TooShort { field: String, min: usize },

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Invalid URL format")]
    InvalidUrl,

    #[error("Invalid slug format (must be lowercase alphanumeric with hyphens)")]
    InvalidSlug,

    #[error("Restricted documents cannot be uploaded")]
    RestrictedDocument,

    #[error("Invalid file type: {mime_type}")]
    InvalidFileType { mime_type: String },

    #[error("File too large (max {max_mb} MB)")]
    FileTooLarge { max_mb: usize },
}

/// Validate a submission creation request
pub fn validate_create_submission(input: &CreateSubmission) -> Result<(), ValidationError> {
    // Submitter name
    if input.submitter_name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "submitter_name".to_string(),
        });
    }
    if input.submitter_name.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "submitter_name".to_string(),
            max: 255,
        });
    }

    // Organization
    if input.organization.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "organization".to_string(),
        });
    }
    if input.organization.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "organization".to_string(),
            max: 255,
        });
    }

    // Email (optional but must be valid if provided)
    if let Some(ref email) = input.submitter_email {
        if !email.is_empty() && !is_valid_email(email) {
            return Err(ValidationError::InvalidEmail);
        }
    }

    // Organization department (optional)
    if let Some(ref dept) = input.organization_department {
        if dept.len() > 255 {
            return Err(ValidationError::TooLong {
                field: "organization_department".to_string(),
                max: 255,
            });
        }
    }

    Ok(())
}

/// Validate an external URL (for wetten.overheid.nl)
pub fn validate_external_url(url: &str) -> Result<(), ValidationError> {
    if url.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "external_url".to_string(),
        });
    }

    // Must be a valid URL
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(ValidationError::InvalidUrl);
    }

    // Should be from wetten.overheid.nl for formal laws
    if !url.contains("wetten.overheid.nl") {
        // Allow for now but could restrict in the future
        tracing::warn!("External URL is not from wetten.overheid.nl: {}", url);
    }

    if url.len() > 2048 {
        return Err(ValidationError::TooLong {
            field: "external_url".to_string(),
            max: 2048,
        });
    }

    Ok(())
}

/// Validate slug format
pub fn validate_slug(slug: &str) -> Result<(), ValidationError> {
    if slug.is_empty() || slug.len() > 50 {
        return Err(ValidationError::InvalidSlug);
    }

    // Must match pattern: lowercase letters, numbers, and hyphens
    let is_valid = slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

    if !is_valid || slug.starts_with('-') || slug.ends_with('-') {
        return Err(ValidationError::InvalidSlug);
    }

    Ok(())
}

/// Check if document classification allows upload
pub fn validate_classification_for_upload(
    classification: DocumentClassification,
) -> Result<(), ValidationError> {
    if classification == DocumentClassification::Restricted {
        return Err(ValidationError::RestrictedDocument);
    }
    Ok(())
}

/// Validate uploaded file
pub fn validate_file_upload(
    mime_type: &str,
    file_size: usize,
    max_size_bytes: usize,
) -> Result<(), ValidationError> {
    // Check file size
    if file_size > max_size_bytes {
        return Err(ValidationError::FileTooLarge {
            max_mb: max_size_bytes / (1024 * 1024),
        });
    }

    // Allowed MIME types
    let allowed_types = [
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.oasis.opendocument.text",
        "text/plain",
        "text/markdown",
        "text/html",
        "application/xml",
        "text/xml",
    ];

    if !allowed_types.contains(&mime_type) {
        return Err(ValidationError::InvalidFileType {
            mime_type: mime_type.to_string(),
        });
    }

    Ok(())
}

/// Simple email validation
fn is_valid_email(email: &str) -> bool {
    // Basic check: contains @ and at least one .
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);

    !local.is_empty() && !domain.is_empty() && domain.contains('.') && domain.len() > 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.nl"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
    }

    #[test]
    fn test_slug_validation() {
        assert!(validate_slug("rr-20240101-abc12").is_ok());
        assert!(validate_slug("my-submission").is_ok());
        assert!(validate_slug("-invalid").is_err());
        assert!(validate_slug("UPPERCASE").is_err());
        assert!(validate_slug("").is_err());
    }
}

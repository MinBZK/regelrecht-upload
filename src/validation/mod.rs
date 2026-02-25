//! Input validation module

use crate::models::{CreateSubmission, DocumentClassification};
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
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

    // Allowed MIME types (no HTML/XML to prevent XSS via stored files)
    let allowed_types = [
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.ms-powerpoint",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "application/vnd.oasis.opendocument.text",
        "application/rtf",
        "text/plain",
        "text/markdown",
        "text/csv",
    ];

    if !allowed_types.contains(&mime_type) {
        return Err(ValidationError::InvalidFileType {
            mime_type: mime_type.to_string(),
        });
    }

    Ok(())
}

/// Dangerous file extensions that could be executed if misconfigured
const DANGEROUS_EXTENSIONS: &[&str] = &[
    // Server-side scripting
    ".php",
    ".phtml",
    ".php3",
    ".php4",
    ".php5",
    ".php7",
    ".phps",
    ".asp",
    ".aspx",
    ".jsp",
    ".jspx",
    ".cgi",
    ".pl",
    ".py",
    ".pyc",
    ".pyo",
    ".rb",
    ".erb",
    // Executables
    ".exe",
    ".bat",
    ".cmd",
    ".com",
    ".msi",
    ".dll",
    ".sh",
    ".bash",
    ".zsh",
    ".ksh",
    // JavaScript/TypeScript (could be dangerous in some contexts)
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".mjs",
    // Server config files
    ".htaccess",
    ".htpasswd",
    // Java
    ".jar",
    ".war",
    ".ear",
    ".class",
];

/// Check filename for dangerous extensions that could be executed
///
/// This is a defense-in-depth measure. Even though:
/// 1. MIME type whitelist blocks most dangerous types
/// 2. Files are not served directly by the web server
///
/// We check for dangerous extensions at the end of the filename
/// and for double extensions (e.g., "malware.php.pdf")
pub fn validate_filename_extensions(filename: &str) -> Result<(), ValidationError> {
    let lower = filename.to_lowercase();

    for ext in DANGEROUS_EXTENSIONS {
        // Check if filename ends with the dangerous extension
        if lower.ends_with(ext) {
            return Err(ValidationError::InvalidFileType {
                mime_type: format!("filename contains dangerous extension: {}", ext),
            });
        }
        // Check for double extensions like .php.pdf (dangerous extension followed by another extension)
        let double_ext_pattern = format!("{}.", ext);
        if lower.contains(&double_ext_pattern) {
            return Err(ValidationError::InvalidFileType {
                mime_type: format!("filename contains dangerous extension: {}", ext),
            });
        }
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

    #[test]
    fn test_validate_create_submission_valid() {
        let input = CreateSubmission {
            submitter_name: "Jan de Vries".to_string(),
            submitter_email: Some("jan@example.com".to_string()),
            organization: "Gemeente Amsterdam".to_string(),
            organization_department: Some("ICT".to_string()),
        };
        assert!(validate_create_submission(&input).is_ok());
    }

    #[test]
    fn test_validate_create_submission_empty_name() {
        let input = CreateSubmission {
            submitter_name: "  ".to_string(),
            submitter_email: None,
            organization: "Org".to_string(),
            organization_department: None,
        };
        assert!(matches!(
            validate_create_submission(&input),
            Err(ValidationError::Required { .. })
        ));
    }

    #[test]
    fn test_validate_create_submission_empty_org() {
        let input = CreateSubmission {
            submitter_name: "Jan".to_string(),
            submitter_email: None,
            organization: "".to_string(),
            organization_department: None,
        };
        assert!(matches!(
            validate_create_submission(&input),
            Err(ValidationError::Required { .. })
        ));
    }

    #[test]
    fn test_validate_create_submission_invalid_email() {
        let input = CreateSubmission {
            submitter_name: "Jan".to_string(),
            submitter_email: Some("not-an-email".to_string()),
            organization: "Org".to_string(),
            organization_department: None,
        };
        assert!(matches!(
            validate_create_submission(&input),
            Err(ValidationError::InvalidEmail)
        ));
    }

    #[test]
    fn test_validate_classification_public() {
        assert!(validate_classification_for_upload(DocumentClassification::Public).is_ok());
    }

    #[test]
    fn test_validate_classification_claude() {
        assert!(validate_classification_for_upload(DocumentClassification::ClaudeAllowed).is_ok());
    }

    #[test]
    fn test_validate_classification_restricted() {
        assert!(matches!(
            validate_classification_for_upload(DocumentClassification::Restricted),
            Err(ValidationError::RestrictedDocument)
        ));
    }

    #[test]
    fn test_validate_external_url_valid() {
        assert!(validate_external_url("https://wetten.overheid.nl/BWBR0001840/2024-01-01").is_ok());
    }

    #[test]
    fn test_validate_external_url_empty() {
        assert!(matches!(
            validate_external_url("  "),
            Err(ValidationError::Required { .. })
        ));
    }

    #[test]
    fn test_validate_external_url_no_protocol() {
        assert!(matches!(
            validate_external_url("wetten.overheid.nl/test"),
            Err(ValidationError::InvalidUrl)
        ));
    }

    #[test]
    fn test_validate_file_upload_valid_pdf() {
        assert!(validate_file_upload("application/pdf", 1024, 50 * 1024 * 1024).is_ok());
    }

    #[test]
    fn test_validate_file_upload_too_large() {
        assert!(matches!(
            validate_file_upload("application/pdf", 100 * 1024 * 1024, 50 * 1024 * 1024),
            Err(ValidationError::FileTooLarge { .. })
        ));
    }

    #[test]
    fn test_validate_file_upload_invalid_type() {
        assert!(matches!(
            validate_file_upload("application/zip", 1024, 50 * 1024 * 1024),
            Err(ValidationError::InvalidFileType { .. })
        ));
    }

    #[test]
    fn test_validate_filename_extensions_safe() {
        assert!(validate_filename_extensions("document.pdf").is_ok());
        assert!(validate_filename_extensions("report.docx").is_ok());
        assert!(validate_filename_extensions("notes.txt").is_ok());
        assert!(validate_filename_extensions("readme.md").is_ok());
    }

    #[test]
    fn test_validate_filename_extensions_dangerous() {
        // Direct dangerous extensions
        assert!(validate_filename_extensions("script.php").is_err());
        assert!(validate_filename_extensions("shell.sh").is_err());
        assert!(validate_filename_extensions("malware.exe").is_err());

        // Double extensions (hidden dangerous extension)
        assert!(validate_filename_extensions("document.php.pdf").is_err());
        assert!(validate_filename_extensions("image.exe.jpg").is_err());

        // Case insensitive
        assert!(validate_filename_extensions("SCRIPT.PHP").is_err());
        assert!(validate_filename_extensions("Shell.SH").is_err());
    }
}

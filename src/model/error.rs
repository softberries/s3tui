//! Structured error types for S3 and local filesystem operations

use std::fmt;

/// Errors that can occur during S3 operations
#[derive(Debug, Clone, PartialEq)]
pub enum S3Error {
    /// Access denied - insufficient permissions
    AccessDenied(String),
    /// Bucket not found
    BucketNotFound(String),
    /// Object/key not found
    ObjectNotFound(String),
    /// Network or connectivity error
    NetworkError(String),
    /// Invalid minio
    InvalidCredentials(String),
    /// Bucket already exists (for creation)
    BucketAlreadyExists(String),
    /// Bucket not empty (for deletion)
    BucketNotEmpty(String),
    /// Generic S3 error
    Other(String),
}

impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Error::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
            S3Error::BucketNotFound(msg) => write!(f, "Bucket not found: {}", msg),
            S3Error::ObjectNotFound(msg) => write!(f, "Object not found: {}", msg),
            S3Error::NetworkError(msg) => write!(f, "Network error: {}", msg),
            S3Error::InvalidCredentials(msg) => write!(f, "Invalid minio: {}", msg),
            S3Error::BucketAlreadyExists(msg) => write!(f, "Bucket already exists: {}", msg),
            S3Error::BucketNotEmpty(msg) => write!(f, "Bucket not empty: {}", msg),
            S3Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl S3Error {
    /// Create an S3Error from an error message, attempting to categorize it
    pub fn from_message(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        let msg_lower = msg.to_lowercase();

        if msg_lower.contains("access denied") || msg_lower.contains("accessdenied") {
            S3Error::AccessDenied(msg)
        } else if msg_lower.contains("no such bucket") || msg_lower.contains("nosuchbucket") {
            S3Error::BucketNotFound(msg)
        } else if msg_lower.contains("no such key") || msg_lower.contains("nosuchkey") {
            S3Error::ObjectNotFound(msg)
        } else if msg_lower.contains("network") || msg_lower.contains("connection") || msg_lower.contains("timeout") {
            S3Error::NetworkError(msg)
        } else if msg_lower.contains("credential") || msg_lower.contains("signature") || msg_lower.contains("unauthorized") {
            S3Error::InvalidCredentials(msg)
        } else if msg_lower.contains("bucket already exists") || msg_lower.contains("bucketalreadyexists") || msg_lower.contains("bucketalreadyownedby") {
            S3Error::BucketAlreadyExists(msg)
        } else if msg_lower.contains("bucket not empty") || msg_lower.contains("bucketnotempty") {
            S3Error::BucketNotEmpty(msg)
        } else {
            S3Error::Other(msg)
        }
    }
}

/// Errors that can occur during local filesystem operations
#[derive(Debug, Clone, PartialEq)]
pub enum LocalError {
    /// File or directory not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Directory not empty (for deletion)
    DirectoryNotEmpty(String),
    /// IO error
    IoError(String),
    /// Generic local error
    Other(String),
}

impl fmt::Display for LocalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalError::NotFound(msg) => write!(f, "Not found: {}", msg),
            LocalError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            LocalError::DirectoryNotEmpty(msg) => write!(f, "Directory not empty: {}", msg),
            LocalError::IoError(msg) => write!(f, "IO error: {}", msg),
            LocalError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl LocalError {
    /// Create a LocalError from an error message, attempting to categorize it
    pub fn from_message(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        let msg_lower = msg.to_lowercase();

        if msg_lower.contains("not found") || msg_lower.contains("no such file") {
            LocalError::NotFound(msg)
        } else if msg_lower.contains("permission denied") || msg_lower.contains("access denied") {
            LocalError::PermissionDenied(msg)
        } else if msg_lower.contains("not empty") || msg_lower.contains("directory not empty") {
            LocalError::DirectoryNotEmpty(msg)
        } else if msg_lower.contains("io error") {
            LocalError::IoError(msg)
        } else {
            LocalError::Other(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_error_from_message_access_denied() {
        let err = S3Error::from_message("Access Denied: you don't have permission");
        assert!(matches!(err, S3Error::AccessDenied(_)));
    }

    #[test]
    fn test_s3_error_from_message_bucket_not_found() {
        let err = S3Error::from_message("NoSuchBucket: bucket does not exist");
        assert!(matches!(err, S3Error::BucketNotFound(_)));
    }

    #[test]
    fn test_s3_error_from_message_other() {
        let err = S3Error::from_message("Some random error");
        assert!(matches!(err, S3Error::Other(_)));
    }

    #[test]
    fn test_local_error_from_message_not_found() {
        let err = LocalError::from_message("No such file or directory");
        assert!(matches!(err, LocalError::NotFound(_)));
    }

    #[test]
    fn test_local_error_from_message_permission_denied() {
        let err = LocalError::from_message("Permission denied: cannot access file");
        assert!(matches!(err, LocalError::PermissionDenied(_)));
    }

    #[test]
    fn test_s3_error_display() {
        let err = S3Error::AccessDenied("test bucket".into());
        assert_eq!(format!("{}", err), "Access denied: test bucket");
    }

    #[test]
    fn test_local_error_display() {
        let err = LocalError::NotFound("/path/to/file".into());
        assert_eq!(format!("{}", err), "Not found: /path/to/file");
    }
}

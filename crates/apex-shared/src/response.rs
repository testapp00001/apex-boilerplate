//! Standardized API response types (RFC 7807 compliant for errors).

use serde::{Deserialize, Serialize};

/// Standard successful API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
        }
    }
}

/// RFC 7807 Problem Details for HTTP APIs.
///
/// See: https://datatracker.ietf.org/doc/html/rfc7807
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// A URI reference that identifies the problem type.
    #[serde(rename = "type")]
    pub error_type: String,

    /// A short, human-readable summary of the problem type.
    pub title: String,

    /// The HTTP status code.
    pub status: u16,

    /// A human-readable explanation specific to this occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// A URI reference that identifies the specific occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Request ID for debugging purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn new(status: u16, title: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".to_string(),
            title: title.into(),
            status,
            detail: None,
            instance: None,
            request_id: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    // Common error constructors
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(400, "Bad Request").with_detail(detail)
    }

    pub fn unauthorized() -> Self {
        Self::new(401, "Unauthorized")
    }

    pub fn forbidden() -> Self {
        Self::new(403, "Forbidden")
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new(404, "Not Found").with_detail(detail)
    }

    pub fn internal_error() -> Self {
        Self::new(500, "Internal Server Error")
    }
}

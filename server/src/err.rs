use serde_json::{json, Value};

pub struct ApiError {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
    pub detail: Value,
}

impl ApiError {
    pub fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            status,
            code,
            message: message.into(),
            detail: Value::Null,
        }
    }

    pub fn detail(mut self, detail: Value) -> Self {
        self.detail = detail;
        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        ApiError::new(422, "validation_error", message)
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        ApiError::new(404, code, message)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        ApiError::new(409, code, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        ApiError::new(500, "internal_error", message)
    }

    pub fn to_json(&self) -> Value {
        let mut error = json!({ "code": self.code, "message": self.message });
        if !self.detail.is_null() {
            error["detail"] = self.detail.clone();
        }
        json!({ "ok": false, "error": error })
    }
}

pub fn validation_field(field: &str, reason: &str, message: impl Into<String>) -> ApiError {
    ApiError::validation(message).detail(json!({ "field": field, "reason": reason }))
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        ApiError::internal(message)
    }
}

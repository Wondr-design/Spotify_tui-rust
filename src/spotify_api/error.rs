#[derive(Debug)]
pub enum ApiError {
    NotAuthenticated(String),
    Request(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotAuthenticated(msg) => write!(f, "not authenticated: {}", msg),
            ApiError::Request(msg) => write!(f, "request failed: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

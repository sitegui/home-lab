macro_rules! unwrap_or_answer {
    ($status:expr, $value:expr) => {{
        match $value {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!("Answering request with {}. Reason: {:?}", $status, error);
                return $status.into_response();
            }
        }
    }};
}

macro_rules! unwrap_or_500 {
    ($value:expr) => {
        unwrap_or_answer!(::axum::http::StatusCode::INTERNAL_SERVER_ERROR, $value)
    };
}

macro_rules! unwrap_or_403 {
    ($value:expr) => {
        unwrap_or_answer!(::axum::http::StatusCode::UNAUTHORIZED, $value)
    };
}

macro_rules! unwrap_or_400 {
    ($value:expr) => {
        unwrap_or_answer!(::axum::http::StatusCode::BAD_REQUEST, $value)
    };
}

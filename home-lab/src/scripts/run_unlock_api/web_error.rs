use anyhow::Error;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub struct WebError {
    status: StatusCode,
    error: Error,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let body = json!({"error": format!("{:?}", self.error)});
        (self.status, Json(body)).into_response()
    }
}

impl From<Error> for WebError {
    fn from(value: Error) -> Self {
        WebError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: value,
        }
    }
}

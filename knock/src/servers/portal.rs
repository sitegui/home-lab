use crate::AppState;
use crate::common::escape_html;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

pub async fn handle_portal_page(State(state): State<Arc<AppState>>) -> Response {
    let config = &state.config;

    let html = unwrap_or_500!(
        config
            .i18n
            .translate(&config.i18n_language, include_str!("../../web/portal.html"))
    );

    let data = serde_json::to_string_pretty(&*state.data.lock()).unwrap_or_default();

    let html = html.replace("{{data}}", &escape_html(&data));

    ([("content-type", "text/html")], html).into_response()
}

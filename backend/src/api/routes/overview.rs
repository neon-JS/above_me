use axum::{extract::State, Json};

use crate::api::{App, state::Overview};

/// Handler for route _/status_
///
/// Responds with an overview of the currently stored states
pub async fn handler(State(app): State<App>) -> Json<Overview> {
    Json(app.get_overview())
}

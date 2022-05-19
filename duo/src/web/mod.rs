use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::Extension,
    handler::Handler,
    http::{StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, get_service},
    Router,
};
use parking_lot::RwLock;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;

use crate::Warehouse;

mod deser;
mod query;
mod routes;
mod serialize;

// Frontend HTML page.
static ROOT_PAGE: Html<&'static str> = Html(include_str!("../../ui/index.html"));

pub struct JaegerData<I: IntoIterator>(pub I);

pub async fn run_web_server(warehouse: Arc<RwLock<Warehouse>>, port: u16) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let layer = ServiceBuilder::new().layer(Extension(warehouse));
    let app = Router::new()
        .route("/", get(|| async { ROOT_PAGE }))
        .nest(
            "/static",
            get_service(ServeDir::new("ui/static")).handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        )
        .route("/api/traces", get(routes::traces))
        .route("/api/traces/:id", get(routes::trace))
        .route("/api/services", get(routes::services))
        .route("/api/services/:service/operations", get(routes::operations))
        .fallback(fallback.into_service())
        .layer(layer);

    println!("Web server listening on http://{}\n", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    let path = uri.path();
    if path.starts_with("/api") || path.starts_with("/static") {
        // For those routes, we simply return 404 text.
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    } else {
        // Due to the frontend is a SPA (Single Page Application),
        // it has own frontend routes, we should return the ROOT PAGE
        // to avoid frontend route 404.
        (StatusCode::TEMPORARY_REDIRECT, ROOT_PAGE).into_response()
    }
}

use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use parking_lot::RwLock;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::{MemoryStore, TraceExt};

use super::deser;
use super::services::{aggregate_span_names, filter_traces, get_trace_by_id};
use super::JaegerData;

#[derive(Debug, Deserialize)]
pub(super) struct QueryParameters {
    pub service: String,
    pub operation: Option<String>,
    #[serde(default, deserialize_with = "deser::option_ignore_error")]
    pub limit: Option<usize>,
    #[serde(default, deserialize_with = "deser::option_miscrosecond")]
    pub start: Option<OffsetDateTime>,
    #[serde(default, deserialize_with = "deser::option_miscrosecond")]
    pub end: Option<OffsetDateTime>,
    #[serde(rename = "maxDuration")]
    #[serde(default, deserialize_with = "deser::option_duration")]
    pub max_duration: Option<Duration>,
    #[serde(rename = "minDuration")]
    #[serde(default, deserialize_with = "deser::option_duration")]
    pub min_duration: Option<Duration>,
}

#[tracing::instrument]
pub(super) async fn list(
    Query(parameters): Query<QueryParameters>,
    Extension(memory_store): Extension<Arc<RwLock<MemoryStore>>>,
) -> impl IntoResponse {
    Json(JaegerData(filter_traces(memory_store, parameters).await))
}

#[tracing::instrument]
pub(super) async fn services(
    Extension(memory_store): Extension<Arc<RwLock<MemoryStore>>>,
) -> impl IntoResponse {
    let memory_store = memory_store.read();
    Json(JaegerData(memory_store.service_names()))
}

#[tracing::instrument]
pub(super) async fn operations(
    Path(service): Path<String>,
    Extension(memory_store): Extension<Arc<RwLock<MemoryStore>>>,
) -> impl IntoResponse {
    Json(JaegerData(
        aggregate_span_names(memory_store, &service).await,
    ))
}

#[tracing::instrument]
pub(super) async fn get_by_id(
    Path(id): Path<String>,
    Extension(memory_store): Extension<Arc<RwLock<MemoryStore>>>,
) -> impl IntoResponse {
    let trace_id = id.parse::<u64>().ok();
    match trace_id {
        Some(trace_id) => {
            if let Some(trace) = get_trace_by_id(memory_store, trace_id).await {
                Json(JaegerData(vec![trace])).into_response()
            } else {
                Json(JaegerData(Vec::<TraceExt>::new())).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, format!("trace {} not found", id)).into_response(),
    }
}

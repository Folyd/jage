use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::response::IntoResponse;
use axum::Json;
use datafusion::prelude::*;
use parking_lot::RwLock;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::query::PartitionQuery;
use crate::Warehouse;

use super::deser;

const DEFAUT_LOG_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub(super) struct QueryParameters {
    pub service: String,
    #[serde(default, deserialize_with = "deser::option_ignore_error")]
    pub limit: Option<usize>,
    #[serde(default, deserialize_with = "deser::option_miscrosecond")]
    pub start: Option<OffsetDateTime>,
    #[serde(default, deserialize_with = "deser::option_miscrosecond")]
    pub end: Option<OffsetDateTime>,
}

#[tracing::instrument]
pub(super) async fn list(
    Query(p): Query<QueryParameters>,
    Extension(warehouse): Extension<Arc<RwLock<Warehouse>>>,
) -> impl IntoResponse {
    let process_prefix = p.service;
    let limit = p.limit.unwrap_or(DEFAUT_LOG_LIMIT);
    let mut total_logs = {
        let warehouse = warehouse.read();
        warehouse
            .logs
            .iter()
            .filter(|log| log.process_id.starts_with(&process_prefix))
            .cloned()
            .collect::<Vec<_>>()
    };
    let pq = if crate::is_memory_mode() || total_logs.len() >= limit {
        None
    } else {
        Some(PartitionQuery::new(
            ".".into(),
            p.start
                .unwrap_or_else(|| OffsetDateTime::now_utc() - Duration::minutes(15)),
            p.end.unwrap_or(OffsetDateTime::now_utc()),
        ))
    };

    if let Some(pq) = pq {
        let expr = col("process_id").like(lit(format!("{process_prefix}%")));
        let logs = pq.query_log(expr).await.unwrap_or_default();
        total_logs.extend(logs);
    }
    Json(total_logs.into_iter().take(limit).collect::<Vec<_>>())
}

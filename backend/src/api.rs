//! Same-origin JSON API and static serving; no frontend build dependency.
use crate::{
    graph::{OSM_ATTRIBUTION, OSM_COPYRIGHT_URL, RoadGraph},
    simulation::{SimulationConfig, SimulationResult, simulate},
};
use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{any, get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path as FilePath,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use uuid::Uuid;

struct StoredRun {
    id: String,
    created: Instant,
    result: Arc<SimulationResult>,
}

pub struct ApiState {
    graph: RoadGraph,
    graph_json: Value,
    version: String,
    runs: Mutex<VecDeque<StoredRun>>,
    gate: Arc<Semaphore>,
    retention: Duration,
}
impl ApiState {
    pub fn new(graph: RoadGraph) -> Self {
        Self::with_retention(graph, Duration::from_secs(900))
    }
    pub fn with_retention(graph: RoadGraph, retention: Duration) -> Self {
        let version = graph.version.clone();
        let graph_json = json!({
            "graph_version":version,
            "coordinate_reference":"WGS84 longitude/latitude (GeoJSON)",
            "nodes":graph.cities,"edges":graph.roads,"boundaries":graph.boundaries,
            "provenance":{
                "boundary_source_url":"https://www.openstreetmap.org/",
                "attribution":OSM_ATTRIBUTION,
                "license_url":OSM_COPYRIGHT_URL,
                "boundary_sha256":graph.boundary_sha256,
                "road_model":"OSM streets clipped to selected cities; simulation links are undirected, weighted by approximate distances between city centers"
            }
        });
        Self {
            graph,
            graph_json,
            version,
            runs: Mutex::new(VecDeque::new()),
            gate: Arc::new(Semaphore::new(1)),
            retention,
        }
    }
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    field: Option<String>,
}
impl ApiError {
    fn new(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        field: Option<&str>,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            field: field.map(str::to_owned),
        }
    }
    fn invalid(message: impl Into<String>, field: Option<&str>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_parameter",
            message,
            field,
        )
    }
    fn missing() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "Result not found or expired",
            None,
        )
    }
    fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Unable to complete request",
            None,
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(json!({"error":{"code":self.code,"message":self.message,"field":self.field}})),
        )
            .into_response();
        if self.status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
        }
        response
    }
}

pub fn router(state: Arc<ApiState>, static_dir: impl AsRef<FilePath>) -> Router {
    let api = Router::new()
        .route("/api/health", get(health))
        .route("/api/graph", get(graph))
        .route(
            "/api/map/roads/{city_id}",
            get(streets).layer(CompressionLayer::new()),
        )
        .route("/api/simulations", post(create_simulation))
        .route("/api/simulations/{id}/encounters", get(encounters))
        .route("/api", any(api_not_found))
        .route("/api/", any(api_not_found))
        .route("/api/{*path}", any(api_not_found))
        .method_not_allowed_fallback(method_not_allowed)
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state);
    Router::new()
        .merge(api)
        .fallback_service(
            ServeDir::new(static_dir.as_ref())
                .fallback(ServeFile::new(static_dir.as_ref().join("index.html"))),
        )
        .layer(TraceLayer::new_for_http())
}
async fn api_not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not_found", "Unknown API path", None)
}
async fn method_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        "Method not allowed",
        None,
    )
}
async fn health(State(state): State<Arc<ApiState>>) -> Json<Value> {
    Json(json!({"status":"ok","graph_version":state.version}))
}
async fn graph(State(state): State<Arc<ApiState>>) -> Json<Value> {
    Json(state.graph_json.clone())
}

async fn streets(
    State(state): State<Arc<ApiState>>,
    Path(city_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state.graph.streets(&city_id).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "City is not in the active map",
            None,
        )
    })?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/geo+json"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        data,
    ))
}

async fn create_simulation(
    State(state): State<Arc<ApiState>>,
    input: Result<Json<SimulationConfig>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(config) = input.map_err(|rejection| {
        let status = rejection.status();
        let code = match status {
            StatusCode::BAD_REQUEST => "malformed_json",
            StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
            StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
            _ => "invalid_parameter",
        };
        ApiError::new(
            status,
            code,
            "Invalid JSON request; check the explicit simulation schema",
            None,
        )
    })?;
    config
        .validate()
        .map_err(|e| ApiError::invalid(e.message, Some(e.field)))?;
    let permit = state.gate.clone().try_acquire_owned().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "A simulation is already running; retry shortly",
            None,
        )
    })?;
    let worker_state = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        // Hold the permit in the worker even if the HTTP client disconnects.
        let _permit = permit;
        simulate(&worker_state.graph, &config)
    })
    .await
    .map_err(|_| ApiError::internal())?
    .map_err(|e| ApiError::invalid(e.message, Some(e.field)))?;
    let id = Uuid::new_v4().to_string();
    let result = Arc::new(result);
    {
        let mut runs = state.runs.lock().map_err(|_| ApiError::internal())?;
        runs.retain(|run| run.created.elapsed() < state.retention);
        while runs.len() >= 4 {
            runs.pop_front();
        }
        runs.push_back(StoredRun {
            id: id.clone(),
            created: Instant::now(),
            result: result.clone(),
        });
    }
    Ok((
        StatusCode::CREATED,
        Json(
            json!({"id":id,"graph_version":state.version,"config":result.config,"summary":result.summary,"agents":result.agents}),
        ),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    cursor: Option<String>,
    limit: Option<usize>,
}
async fn encounters(
    State(state): State<Arc<ApiState>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Path(id) = path.map_err(|_| ApiError::invalid("Invalid simulation ID", Some("id")))?;
    let Query(query) = query.map_err(|_| ApiError::invalid("Invalid pagination query", None))?;
    let limit = query.limit.unwrap_or(100);
    if !(1..=1000).contains(&limit) {
        return Err(ApiError::invalid(
            "Must be between 1 and 1000",
            Some("limit"),
        ));
    }
    let result = {
        let mut runs = state.runs.lock().map_err(|_| ApiError::internal())?;
        runs.retain(|run| run.created.elapsed() < state.retention);
        runs.iter()
            .find(|run| run.id == id)
            .map(|run| run.result.clone())
            .ok_or_else(ApiError::missing)?
    };
    let offset = match query.cursor {
        None => 0,
        Some(cursor) => cursor
            .strip_prefix(&format!("{id}:"))
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| {
                ApiError::invalid("Invalid cursor for this simulation", Some("cursor"))
            })?,
    };
    if offset > result.encounters.len() {
        return Err(ApiError::invalid("Cursor outside result", Some("cursor")));
    }
    let end = offset.saturating_add(limit).min(result.encounters.len());
    let next_cursor = (end < result.encounters.len()).then(|| format!("{id}:{end}"));
    Ok(Json(
        json!({"simulation_id":id,"events":result.encounters[offset..end],"next_cursor":next_cursor}),
    ))
}

pub async fn serve(
    address: std::net::SocketAddr,
    static_dir: std::path::PathBuf,
    graph: RoadGraph,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ApiState::new(graph));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address,"MeetAgain listening");
    axum::serve(listener, router(state, static_dir))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn concurrency_guard_returns_json_429() {
        let state = Arc::new(ApiState::new(RoadGraph::bundled().unwrap()));
        let _permit = state.gate.acquire().await.unwrap();
        let config = json!({"agents":20,"ticks":10,"seed":"1","movement":{"home_move_probability":0.1,"away_move_probability":0.25,"home_return_probability":0.6},"encounters":{"include_tick_zero":false,"cross_tick_tolerance":0}});
        let response = router(state.clone(), "/tmp/missing-static")
            .oneshot(
                Request::post("/api/simulations")
                    .header("content-type", "application/json")
                    .body(Body::from(config.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 429);
        assert_eq!(response.headers()[header::RETRY_AFTER], "1");
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["error"]["code"], "busy");
    }
}

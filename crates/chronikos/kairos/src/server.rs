//! HTTP server for Kairos time management service.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::config::Config;
use crate::schedule::*;
use crate::scheduler::Scheduler;
use crate::store::{calculate_initial_next_fire, ScheduleStore};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<ScheduleStore>,
}

/// HTTP server for Kairos.
pub struct KairosServer {
    config: Config,
    state: Arc<AppState>,
}

impl KairosServer {
    /// Creates a new server instance.
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "kairos=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Initializing Kairos time management service");

        let store = Arc::new(ScheduleStore::new(&config.database_path).await?);
        let state = Arc::new(AppState { store });

        Ok(Self { config, state })
    }

    /// Starts the server.
    pub async fn run(self) -> anyhow::Result<()> {
        use tower_http::{
            cors::{Any, CorsLayer},
            trace::TraceLayer,
        };

        // Spawn the scheduler
        let scheduler = Scheduler::new(self.state.store.clone(), self.config.tick_interval_ms);
        tokio::spawn(scheduler.run());

        let app = Router::new()
            .route("/health", get(health_check))
            .route("/status", get(get_status))
            // Schedule routes
            .route("/schedules", post(create_schedule))
            .route("/schedules", get(list_schedules))
            .route("/schedules/next", get(get_next_schedule))
            .route("/schedules/triggered", get(get_triggered))
            .route("/schedules/triggered/ack", post(ack_triggered))
            .route("/schedules/{id}", get(get_schedule))
            .route("/schedules/{id}", delete(delete_schedule))
            .route("/schedules/{id}", patch(update_schedule))
            .with_state((*self.state).clone())
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(TraceLayer::new_for_http());

        let bind_address = self.config.bind_address();
        let addr: SocketAddr = bind_address
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        info!("Starting Kairos server on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to address: {}", e))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}

/// Query parameters for listing schedules.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub tag: Option<String>,
    pub status: Option<String>,
}

/// Get service status.
async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.get_stats().await {
        Ok((active, pending, next_fire)) => Json(StatusResponse {
            healthy: true,
            active_schedules: active,
            pending_triggered: pending,
            next_fire,
        }),
        Err(e) => {
            tracing::error!("Failed to get stats: {}", e);
            Json(StatusResponse {
                healthy: false,
                active_schedules: 0,
                pending_triggered: 0,
                next_fire: None,
            })
        }
    }
}

/// Create a new schedule.
async fn create_schedule(
    State(state): State<AppState>,
    Json(req): Json<CreateScheduleRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let now = time::OffsetDateTime::now_utc();

    let next_fire = match calculate_initial_next_fire(&req.trigger, now) {
        Ok(t) => Some(t),
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            );
        }
    };

    let schedule = Schedule {
        id: id.clone(),
        name: req.name,
        trigger: req.trigger,
        payload: req.payload,
        tags: req.tags,
        priority: req.priority,
        status: ScheduleStatus::Active,
        created_at: now,
        next_fire,
        last_fire: None,
    };

    match state.store.create(&schedule).await {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::to_value(schedule).unwrap())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// List schedules.
async fn list_schedules(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let status = query.status.and_then(|s| match s.as_str() {
        "active" => Some(ScheduleStatus::Active),
        "paused" => Some(ScheduleStatus::Paused),
        "completed" => Some(ScheduleStatus::Completed),
        "triggered" => Some(ScheduleStatus::Triggered),
        _ => None,
    });

    match state.store.list(status, query.tag.as_deref()).await {
        Ok(schedules) => {
            let total = schedules.len();
            Json(SchedulesListResponse { schedules, total })
        }
        Err(_e) => Json(SchedulesListResponse {
            schedules: vec![],
            total: 0,
        }),
    }
}

/// Get the next schedule to fire.
async fn get_next_schedule(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.get_next().await {
        Ok(Some(schedule)) => (
            StatusCode::OK,
            Json(Some(serde_json::to_value(schedule).unwrap())),
        ),
        Ok(None) => (StatusCode::OK, Json(None)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Some(serde_json::json!({ "error": e.to_string() }))),
        ),
    }
}

/// Get triggered schedules (for kairos-herald).
async fn get_triggered(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.get_triggered().await {
        Ok(triggered) => Json(triggered),
        Err(_) => Json(vec![]),
    }
}

/// Acknowledge triggered schedules.
async fn ack_triggered(
    State(state): State<AppState>,
    Json(req): Json<AckTriggeredRequest>,
) -> impl IntoResponse {
    match state.store.ack_triggered(&req.ids).await {
        Ok(count) => (
            StatusCode::OK,
            Json(serde_json::json!({ "acknowledged": count })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// Get a specific schedule.
async fn get_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.store.get(&id).await {
        Ok(Some(schedule)) => (StatusCode::OK, Json(Some(schedule))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(None)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

/// Delete a schedule.
async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.store.delete(&id).await {
        Ok(true) => (StatusCode::NO_CONTENT, Json(serde_json::json!({}))),
        Ok(false) => (StatusCode::NOT_FOUND, Json(serde_json::json!({}))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({}))),
    }
}

/// Update a schedule.
async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> impl IntoResponse {
    if let Some(status) = req.status {
        match state.store.update_status(&id, status).await {
            Ok(_) => {
                // Return updated schedule
                match state.store.get(&id).await {
                    Ok(Some(schedule)) => {
                        return (StatusCode::OK, Json(serde_json::to_value(schedule).unwrap()));
                    }
                    _ => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "error": "schedule not found after update" })),
                        );
                    }
                }
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                );
            }
        }
    }

    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "no update specified" })))
}

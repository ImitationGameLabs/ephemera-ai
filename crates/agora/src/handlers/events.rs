//! Event HTTP handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{error, info, instrument};

use crate::event::{
    BatchUpdateEventsRequest, BatchUpdateEventsResponse, CreateEventRequest, Event,
    EventsListResponse, EventStatus, UpdateEventRequest,
};
use crate::server::AppState;

/// Query parameters for GET /events.
#[derive(Debug, Deserialize)]
pub struct GetEventsQuery {
    /// Filter by status.
    pub status: Option<EventStatus>,
    /// Maximum number of events to return.
    pub limit: Option<u32>,
}

/// HTTP handler for event operations.
pub struct EventHandler;

impl EventHandler {
    /// Push a new event (POST /events).
    #[instrument(skip(state))]
    pub async fn create(
        State(state): State<AppState>,
        Json(request): Json<CreateEventRequest>,
    ) -> Result<Json<Event>, StatusCode> {
        info!(
            "Creating event: type={}, herald={}",
            request.event_type, request.herald_id
        );

        let event = state.event_queue.push(request).await;
        info!("Created event with id={}", event.id);
        Ok(Json(event))
    }

    /// Get pending events (GET /events).
    #[instrument(skip(state))]
    pub async fn list(
        State(state): State<AppState>,
        Query(query): Query<GetEventsQuery>,
    ) -> Result<Json<EventsListResponse>, StatusCode> {
        info!(
            "Getting events: status={:?}, limit={:?}",
            query.status, query.limit
        );

        let events = state
            .event_queue
            .get_by_status(query.status, query.limit)
            .await;

        let total = events.len();
        info!("Found {} events", total);

        Ok(Json(EventsListResponse { events, total }))
    }

    /// Update event status (PATCH /events/{id}).
    #[instrument(skip(state))]
    pub async fn update(
        State(state): State<AppState>,
        Path(id): Path<u64>,
        Json(request): Json<UpdateEventRequest>,
    ) -> Result<Json<Event>, StatusCode> {
        info!("Updating event {}: status={:?}", id, request.status);

        match state.event_queue.update_status(id, request.status).await {
            Some(event) => {
                info!("Updated event {}", id);
                Ok(Json(event))
            }
            None => {
                error!("Event {} not found", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Batch update event status (PATCH /events).
    #[instrument(skip(state))]
    pub async fn batch_update(
        State(state): State<AppState>,
        Json(request): Json<BatchUpdateEventsRequest>,
    ) -> Result<Json<BatchUpdateEventsResponse>, StatusCode> {
        info!(
            "Batch updating {} events to status={:?}",
            request.event_ids.len(),
            request.status
        );

        let updated = state
            .event_queue
            .batch_update_status(request.event_ids, request.status)
            .await;

        info!("Updated {} events", updated);
        Ok(Json(BatchUpdateEventsResponse { updated }))
    }
}

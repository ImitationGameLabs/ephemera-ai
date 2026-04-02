//! Event HTTP handlers.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{error, info, instrument, warn};

use crate::event::{
    BatchUpdateEventsRequest, BatchUpdateEventsResponse, CreateEventRequest, Event, EventStatus,
    EventsListResponse, UpdateEventRequest,
};
use crate::server::AppState;

/// Request body for POST /events/fetch.
#[derive(Debug, Deserialize)]
pub struct FetchEventsRequest {
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

        match state.event_queue.push(request).await {
            Ok(event) => {
                info!("Created event with id={}", event.id);
                Ok(Json(event))
            }
            Err(e) => {
                error!("Failed to create event: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Fetch events for delivery (POST /events/fetch).
    /// Changes state: Pending → Delivered.
    #[instrument(skip(state))]
    pub async fn fetch(
        State(state): State<AppState>,
        Json(query): Json<FetchEventsRequest>,
    ) -> Result<Json<EventsListResponse>, StatusCode> {
        info!("Fetching events: limit={}", query.limit.unwrap_or(10));

        let events = state.event_queue.fetch(query.limit.unwrap_or(10)).await;

        let total = events.len();
        info!("Fetched {} events", total);

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

        // Only allow acking events
        if request.status != EventStatus::Acked {
            warn!("Rejecting non-ack status update for event {}", id);
            return Err(StatusCode::BAD_REQUEST);
        }

        match state.event_queue.update_status(id, request.status).await {
            Ok(Some(event)) => {
                info!("Acked event {}", id);
                Ok(Json(event))
            }
            Ok(None) => {
                warn!("Event {} not found for ack", id);
                Err(StatusCode::NOT_FOUND)
            }
            Err(e) => {
                error!("Failed to ack event {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
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

        // Only allow acking events (consistent with single update)
        if request.status != EventStatus::Acked {
            warn!(
                "Rejecting batch update with non-ack status {:?}",
                request.status
            );
            return Err(StatusCode::BAD_REQUEST);
        }

        match state
            .event_queue
            .batch_update_status(request.event_ids, request.status)
            .await
        {
            Ok(acked_ids) => {
                info!("Batch acked {} events: {:?}", acked_ids.len(), acked_ids);
                Ok(Json(BatchUpdateEventsResponse { acked_ids }))
            }
            Err(e) => {
                error!("Failed to batch update events: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

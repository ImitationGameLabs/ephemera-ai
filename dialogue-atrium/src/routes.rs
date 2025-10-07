use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower::ServiceBuilder;
use crate::handlers::{
    create_user, get_user_profile, get_own_profile, update_profile, get_all_users,
    update_heartbeat,
    create_message, get_messages, get_message, delete_message
};
use crate::db::{UserManager, MessageManager};

pub fn create_routes(user_manager: UserManager, message_manager: MessageManager) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/v1", Router::new()
            // User routes
            .route("/users", post(create_user).get(get_all_users))
            .route("/users/{username}", get(get_user_profile))
            .route("/profile", get(get_own_profile).put(update_profile))
            .with_state(user_manager.clone())
        )
        .nest("/api/v1", Router::new()
            // Heartbeat route
            .route("/heartbeat", put(update_heartbeat))
            .with_state((user_manager.clone(), message_manager.clone()))
        )
        .nest("/api/v1", Router::new()
            // Message routes
            .route("/messages", get(get_messages))
            .route("/messages/{id}", get(get_message).delete(delete_message))
            .with_state(message_manager.clone())
        )
        .nest("/api/v1", Router::new()
            // Create message route
            .route("/messages", post(create_message))
            .with_state((message_manager, user_manager))
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
}
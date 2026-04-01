//! Mock Agora client for testing
//!
//! This mock implementation allows unit tests to verify behavior
//! without requiring a real Agora service.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use agora_common::event::Event;
use agora_common::herald::HeraldInfo;

use crate::{AgoraClientError, AgoraClientTrait};

/// Record of a method call for verification
#[derive(Debug, Clone, PartialEq)]
pub enum MockCall {
    HealthCheck,
    FetchEvents { limit: Option<u32> },
    AckEvent { event_id: u64 },
    AckEvents { event_ids: Vec<u64> },
    ListHeralds,
    GetHerald { id: String },
}

/// Mock response types
#[derive(Debug, Clone)]
pub enum MockResponse {
    HealthCheck(String),
    Events(Vec<Event>),
    Event(Event),
    AckCount(usize),
    Heralds(Vec<HeraldInfo>),
    Herald(HeraldInfo),
    Empty,
}

/// Internal state for the mock client
#[derive(Debug, Default)]
struct MockState {
    /// Queue of responses to return for subsequent calls
    responses: VecDeque<Result<MockResponse, String>>,
    /// All calls made to this mock
    calls: Vec<MockCall>,
}

/// Mock Agora client for testing
#[derive(Debug)]
pub struct MockAgoraClient {
    /// Base URL for the mock client
    base_url: String,
    /// Internal state protected by mutex for interior mutability
    state: Arc<Mutex<MockState>>,
    /// Default events response when queues are empty
    default_events_response: Vec<Event>,
}

impl Default for MockAgoraClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAgoraClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            base_url: "http://mock-agora".to_string(),
            state: Arc::new(Mutex::new(MockState::default())),
            default_events_response: vec![],
        }
    }

    /// Create a new mock client with a specific base URL
    pub fn with_base_url(base_url: &str) -> Self {
        let mut client = Self::new();
        client.base_url = base_url.to_string();
        client
    }

    /// Add a health check response to the queue
    pub fn push_health_check(&mut self, response: impl Into<String>) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::HealthCheck(response.into())));
        self
    }

    /// Add an events response to the queue
    pub fn push_events(&mut self, events: Vec<Event>) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::Events(events)));
        self
    }

    /// Add a single event response to the queue
    pub fn push_event(&mut self, event: Event) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::Event(event)));
        self
    }

    /// Add an ack count response to the queue
    pub fn push_ack_count(&mut self, count: usize) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::AckCount(count)));
        self
    }

    /// Add a heralds response to the queue
    pub fn push_heralds(&mut self, heralds: Vec<HeraldInfo>) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::Heralds(heralds)));
        self
    }

    /// Add a herald response to the queue
    pub fn push_herald(&mut self, herald: HeraldInfo) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::Herald(herald)));
        self
    }

    /// Add an empty success response to the queue
    pub fn push_empty(&mut self) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Ok(MockResponse::Empty));
        self
    }

    /// Add an error response to the queue
    pub fn push_error(&mut self, error: impl Into<String>) -> &mut Self {
        self.state.lock().unwrap().responses.push_back(Err(error.into()));
        self
    }

    /// Set the default events response when queues are empty
    pub fn set_default_events(&mut self, events: Vec<Event>) -> &mut Self {
        self.default_events_response = events;
        self
    }

    /// Get all calls made to this mock
    pub fn get_calls(&self) -> Vec<MockCall> {
        self.state.lock().unwrap().calls.clone()
    }

    /// Check if a specific method was called
    pub fn was_called(&self, check: impl Fn(&MockCall) -> bool) -> bool {
        self.state.lock().unwrap().calls.iter().any(check)
    }

    /// Get the count of calls matching a predicate
    pub fn call_count(&self, check: impl Fn(&MockCall) -> bool) -> usize {
        self.state.lock().unwrap().calls.iter().filter(|c| check(c)).count()
    }

    /// Clear all recorded calls
    pub fn clear_calls(&self) {
        self.state.lock().unwrap().calls.clear();
    }

    /// Record a call
    fn record_call(&self, call: MockCall) {
        self.state.lock().unwrap().calls.push(call);
    }

    /// Get next response from queue
    fn pop_response(&self) -> Option<Result<MockResponse, String>> {
        self.state.lock().unwrap().responses.pop_front()
    }
}

#[async_trait]
impl AgoraClientTrait for MockAgoraClient {
    async fn health_check(&self) -> Result<String, AgoraClientError> {
        self.record_call(MockCall::HealthCheck);
        match self.pop_response() {
            Some(Ok(MockResponse::HealthCheck(s))) => Ok(s),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Ok("OK".to_string()),
        }
    }

    async fn fetch_events(&self, limit: Option<u32>) -> Result<Vec<Event>, AgoraClientError> {
        self.record_call(MockCall::FetchEvents { limit });
        match self.pop_response() {
            Some(Ok(MockResponse::Events(events))) => Ok(events),
            Some(Ok(MockResponse::Empty)) => Ok(vec![]),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Ok(self.default_events_response.clone()),
        }
    }

    // Design note: Single-value methods require explicit response configuration.
    // Unlike list methods that can return empty Vec as a sensible default,
    // single-value methods cannot create a meaningful default value.
    // Always use push_event() before calling ack_event() in tests.
    async fn ack_event(&self, event_id: u64) -> Result<Event, AgoraClientError> {
        self.record_call(MockCall::AckEvent { event_id });
        match self.pop_response() {
            Some(Ok(MockResponse::Event(event))) => Ok(event),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Err(AgoraClientError::ApiError("No response configured for ack_event".to_string())),
        }
    }

    async fn ack_events(&self, event_ids: Vec<u64>) -> Result<usize, AgoraClientError> {
        self.record_call(MockCall::AckEvents { event_ids });
        match self.pop_response() {
            Some(Ok(MockResponse::AckCount(count))) => Ok(count),
            Some(Ok(MockResponse::Empty)) => Ok(0),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Ok(0),
        }
    }

    async fn list_heralds(&self) -> Result<Vec<HeraldInfo>, AgoraClientError> {
        self.record_call(MockCall::ListHeralds);
        match self.pop_response() {
            Some(Ok(MockResponse::Heralds(heralds))) => Ok(heralds),
            Some(Ok(MockResponse::Empty)) => Ok(vec![]),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Ok(vec![]),
        }
    }

    // Design note: Single-value methods require explicit response configuration.
    // Unlike list methods that can return empty Vec as a sensible default,
    // single-value methods cannot create a meaningful default value.
    // Always use push_herald() before calling get_herald() in tests.
    async fn get_herald(&self, id: &str) -> Result<HeraldInfo, AgoraClientError> {
        self.record_call(MockCall::GetHerald { id: id.to_string() });
        match self.pop_response() {
            Some(Ok(MockResponse::Herald(herald))) => Ok(herald),
            Some(Err(e)) => Err(AgoraClientError::ApiError(e)),
            _ => Err(AgoraClientError::ApiError("No response configured for get_herald".to_string())),
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agora_common::event::{EventPriority, EventStatus};
    use agora_common::herald::HeraldStatus;

    #[test]
    fn test_mock_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockAgoraClient>();
    }

    fn create_test_event(id: u64) -> Event {
        Event {
            id,
            event_type: "test.event".to_string(),
            herald_id: "test-herald".to_string(),
            payload: serde_json::json!({}),
            priority: EventPriority::Normal,
            timestamp: time::OffsetDateTime::now_utc(),
            status: EventStatus::Pending,
        }
    }

    fn create_test_herald(id: &str) -> HeraldInfo {
        let now = time::OffsetDateTime::now_utc();
        HeraldInfo {
            id: id.to_string(),
            description: Some(format!("Test herald {}", id)),
            status: HeraldStatus::Active,
            registered_at: now,
            last_heartbeat: now,
        }
    }

    #[tokio::test]
    async fn test_mock_health_check() {
        let mut mock = MockAgoraClient::new();
        mock.push_health_check("healthy");

        let result = mock.health_check().await.unwrap();
        assert_eq!(result, "healthy");

        assert!(mock.was_called(|c| matches!(c, MockCall::HealthCheck)));
    }

    #[tokio::test]
    async fn test_mock_fetch_events() {
        let mut mock = MockAgoraClient::new();
        let event = create_test_event(1);
        mock.push_events(vec![event.clone()]);

        let result = mock.fetch_events(Some(10)).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);

        assert!(mock.was_called(|c| matches!(c, MockCall::FetchEvents { .. })));
    }

    #[tokio::test]
    async fn test_mock_ack_event() {
        let mut mock = MockAgoraClient::new();
        let event = create_test_event(42);
        mock.push_event(event);

        let result = mock.ack_event(42).await.unwrap();
        assert_eq!(result.id, 42);

        assert!(mock.was_called(|c| matches!(c, MockCall::AckEvent { event_id: 42 })));
    }

    #[tokio::test]
    async fn test_mock_ack_events() {
        let mut mock = MockAgoraClient::new();
        mock.push_ack_count(3);

        let result = mock.ack_events(vec![1, 2, 3]).await.unwrap();
        assert_eq!(result, 3);

        assert!(mock.was_called(|c| matches!(c, MockCall::AckEvents { .. })));
    }

    #[tokio::test]
    async fn test_mock_list_heralds() {
        let mut mock = MockAgoraClient::new();
        let herald = create_test_herald("herald-1");
        mock.push_heralds(vec![herald]);

        let result = mock.list_heralds().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "herald-1");

        assert!(mock.was_called(|c| matches!(c, MockCall::ListHeralds)));
    }

    #[tokio::test]
    async fn test_mock_get_herald() {
        let mut mock = MockAgoraClient::new();
        let herald = create_test_herald("herald-42");
        mock.push_herald(herald);

        let result = mock.get_herald("herald-42").await.unwrap();
        assert_eq!(result.id, "herald-42");

        assert!(mock.was_called(|c| matches!(c, MockCall::GetHerald { id } if id == "herald-42")));
    }

    #[tokio::test]
    async fn test_mock_error_response() {
        let mut mock = MockAgoraClient::new();
        mock.push_error("Service unavailable");

        let result = mock.fetch_events(None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgoraClientError::ApiError(_)));
    }

    #[tokio::test]
    async fn test_mock_default_response() {
        let mut mock = MockAgoraClient::new();
        let event = create_test_event(99);
        mock.set_default_events(vec![event]);

        // No response pushed, should use default
        let result = mock.fetch_events(None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 99);
    }

    #[tokio::test]
    async fn test_mock_clear_calls() {
        let mut mock = MockAgoraClient::new();
        mock.push_health_check("ok");

        let _ = mock.health_check().await;
        assert_eq!(mock.get_calls().len(), 1);

        mock.clear_calls();
        assert!(mock.get_calls().is_empty());
    }

    #[tokio::test]
    async fn test_mock_sequential_responses() {
        let mut mock = MockAgoraClient::new();

        // Push multiple different responses
        mock.push_events(vec![create_test_event(1)])
            .push_events(vec![create_test_event(2), create_test_event(3)])
            .push_error("third call fails");

        // First call returns first batch
        let result1 = mock.fetch_events(None).await.unwrap();
        assert_eq!(result1.len(), 1);
        assert_eq!(result1[0].id, 1);

        // Second call returns second batch
        let result2 = mock.fetch_events(None).await.unwrap();
        assert_eq!(result2.len(), 2);
        assert_eq!(result2[0].id, 2);

        // Third call returns error
        let result3 = mock.fetch_events(None).await;
        assert!(result3.is_err());
    }

    #[tokio::test]
    async fn test_mock_with_trait_object() {
        // Test that mock works through Arc<dyn AgoraClientTrait>
        let mut mock = MockAgoraClient::new();
        mock.push_health_check("healthy");

        let client: Arc<dyn AgoraClientTrait> = Arc::new(mock);

        let result = client.health_check().await.unwrap();
        assert_eq!(result, "healthy");
    }

    #[tokio::test]
    async fn test_mock_fetch_events_with_limit() {
        let mut mock = MockAgoraClient::new();
        mock.push_events(vec![create_test_event(1)]);

        let result = mock.fetch_events(Some(10)).await.unwrap();
        assert_eq!(result.len(), 1);

        // Verify the limit parameter was recorded
        assert!(mock.was_called(|c| matches!(c, MockCall::FetchEvents { limit: Some(10) })));
    }

    #[tokio::test]
    async fn test_mock_multiple_ack_calls() {
        let mut mock = MockAgoraClient::new();
        mock.push_ack_count(1)
            .push_ack_count(3);

        // Ack single event
        let result1 = mock.ack_events(vec![1]).await.unwrap();
        assert_eq!(result1, 1);

        // Ack multiple events
        let result2 = mock.ack_events(vec![2, 3, 4]).await.unwrap();
        assert_eq!(result2, 3);

        // Verify both calls with exact parameters using PartialEq
        assert_eq!(
            mock.get_calls(),
            vec![
                MockCall::AckEvents { event_ids: vec![1] },
                MockCall::AckEvents { event_ids: vec![2, 3, 4] }
            ]
        );
    }

    #[tokio::test]
    async fn test_mock_chain_configuration() {
        let mut mock = MockAgoraClient::new();
        mock.push_health_check("ok")
            .push_events(vec![create_test_event(1)])
            .push_ack_count(1);

        let _ = mock.health_check().await.unwrap();
        let _ = mock.fetch_events(None).await.unwrap();
        let _ = mock.ack_events(vec![1]).await.unwrap();

        // Verify exact call sequence using PartialEq
        assert_eq!(
            mock.get_calls(),
            vec![
                MockCall::HealthCheck,
                MockCall::FetchEvents { limit: None },
                MockCall::AckEvents { event_ids: vec![1] }
            ]
        );
    }

    #[tokio::test]
    async fn test_mock_empty_response_variants() {
        let mut mock = MockAgoraClient::new();

        // Empty response should return empty vec for fetch_events
        mock.push_empty();
        let result = mock.fetch_events(None).await.unwrap();
        assert!(result.is_empty());

        // Empty response should return empty vec for list_heralds
        mock.push_empty();
        let result = mock.list_heralds().await.unwrap();
        assert!(result.is_empty());
    }
}

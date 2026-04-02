//! Mock Loom client for testing
//!
//! This mock implementation allows unit tests to verify behavior
//! without requiring a real Loom service.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use loom_common::models::*;
use loom_common::types::MemoryFragment;

use crate::{LoomClientError, LoomClientTrait};

/// Record of a method call for verification
#[derive(Debug, Clone, PartialEq)]
pub enum MockCall {
    HealthCheck,
    CreateMemory { fragments_count: usize },
    CreateSingleMemory { fragment_id: i64 },
    GetMemory { id: i64 },
    DeleteMemory { id: i64 },
    GetRecentMemories { limit: usize },
    GetTimelineMemory { from: String, to: String, limit: Option<usize>, offset: Option<usize> },
    GetPinnedMemories,
    PinMemory { memory_id: i64, reason: Option<String> },
    UnpinMemory { memory_id: i64 },
}

/// Mock response types
#[derive(Debug, Clone)]
pub enum MockResponse {
    HealthCheck(serde_json::Value),
    Memory(MemoryResponse),
    PinnedMemories(PinnedMemoriesResponse),
    PinnedMemory(PinnedMemory),
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

/// Mock Loom client for testing
#[derive(Debug)]
pub struct MockLoomClient {
    /// Base URL for the mock client
    base_url: String,
    /// Internal state protected by mutex for interior mutability
    state: Arc<Mutex<MockState>>,
    /// Default memory response when queues are empty
    default_memory_response: MemoryResponse,
    /// Default pinned memories response
    default_pinned_response: PinnedMemoriesResponse,
}

impl Default for MockLoomClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLoomClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            base_url: "http://mock-loom".to_string(),
            state: Arc::new(Mutex::new(MockState::default())),
            default_memory_response: MemoryResponse { fragments: vec![], total: 0 },
            default_pinned_response: PinnedMemoriesResponse { items: vec![] },
        }
    }

    /// Create a new mock client with a specific base URL
    pub fn with_base_url(base_url: &str) -> Self {
        let mut client = Self::new();
        client.base_url = base_url.to_string();
        client
    }

    /// Add a health check response to the queue
    pub fn push_health_check(&mut self, value: serde_json::Value) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Ok(MockResponse::HealthCheck(value)));
        self
    }

    /// Add a memory response to the queue
    pub fn push_memory(&mut self, response: MemoryResponse) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Ok(MockResponse::Memory(response)));
        self
    }

    /// Add a pinned memories response to the queue
    pub fn push_pinned_memories(&mut self, response: PinnedMemoriesResponse) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Ok(MockResponse::PinnedMemories(response)));
        self
    }

    /// Add a pinned memory response to the queue
    pub fn push_pinned_memory(&mut self, pinned: PinnedMemory) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Ok(MockResponse::PinnedMemory(pinned)));
        self
    }

    /// Add an empty success response to the queue
    pub fn push_empty(&mut self) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Ok(MockResponse::Empty));
        self
    }

    /// Add an error response to the queue
    pub fn push_error(&mut self, error: impl Into<String>) -> &mut Self {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(Err(error.into()));
        self
    }

    /// Set the default memory response when queues are empty
    pub fn set_default_memory(&mut self, response: MemoryResponse) -> &mut Self {
        self.default_memory_response = response;
        self
    }

    /// Set the default pinned memories response when queues are empty
    pub fn set_default_pinned(&mut self, response: PinnedMemoriesResponse) -> &mut Self {
        self.default_pinned_response = response;
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
        self.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|c| check(c))
            .count()
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
impl LoomClientTrait for MockLoomClient {
    async fn health_check(&self) -> Result<serde_json::Value, LoomClientError> {
        self.record_call(MockCall::HealthCheck);
        match self.pop_response() {
            Some(Ok(MockResponse::HealthCheck(v))) => Ok(v),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(serde_json::json!({"status": "ok"})),
        }
    }

    async fn create_memory(
        &self,
        request: CreateMemoryRequest,
    ) -> Result<MemoryResponse, LoomClientError> {
        self.record_call(MockCall::CreateMemory { fragments_count: request.fragments.len() });
        match self.pop_response() {
            Some(Ok(MockResponse::Memory(m))) => Ok(m),
            Some(Ok(MockResponse::Empty)) => Ok(MemoryResponse { fragments: vec![], total: 0 }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_memory_response.clone()),
        }
    }

    async fn create_single_memory(
        &self,
        fragment: MemoryFragment,
    ) -> Result<MemoryResponse, LoomClientError> {
        self.record_call(MockCall::CreateSingleMemory { fragment_id: fragment.id });
        match self.pop_response() {
            Some(Ok(MockResponse::Memory(m))) => Ok(m),
            Some(Ok(MockResponse::Empty)) => Ok(MemoryResponse { fragments: vec![], total: 0 }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_memory_response.clone()),
        }
    }

    async fn get_memory(&self, id: i64) -> Result<MemoryResponse, LoomClientError> {
        self.record_call(MockCall::GetMemory { id });
        match self.pop_response() {
            Some(Ok(MockResponse::Memory(m))) => Ok(m),
            Some(Ok(MockResponse::Empty)) => Ok(MemoryResponse { fragments: vec![], total: 0 }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_memory_response.clone()),
        }
    }

    async fn delete_memory(&self, id: i64) -> Result<(), LoomClientError> {
        self.record_call(MockCall::DeleteMemory { id });
        match self.pop_response() {
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(()),
        }
    }

    async fn get_recent_memories(&self, limit: usize) -> Result<MemoryResponse, LoomClientError> {
        self.record_call(MockCall::GetRecentMemories { limit });
        match self.pop_response() {
            Some(Ok(MockResponse::Memory(m))) => Ok(m),
            Some(Ok(MockResponse::Empty)) => Ok(MemoryResponse { fragments: vec![], total: 0 }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_memory_response.clone()),
        }
    }

    async fn get_timeline_memory(
        &self,
        from: &str,
        to: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<MemoryResponse, LoomClientError> {
        self.record_call(MockCall::GetTimelineMemory {
            from: from.to_string(),
            to: to.to_string(),
            limit,
            offset,
        });
        match self.pop_response() {
            Some(Ok(MockResponse::Memory(m))) => Ok(m),
            Some(Ok(MockResponse::Empty)) => Ok(MemoryResponse { fragments: vec![], total: 0 }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_memory_response.clone()),
        }
    }

    async fn get_pinned_memories(&self) -> Result<PinnedMemoriesResponse, LoomClientError> {
        self.record_call(MockCall::GetPinnedMemories);
        match self.pop_response() {
            Some(Ok(MockResponse::PinnedMemories(p))) => Ok(p),
            Some(Ok(MockResponse::Empty)) => Ok(PinnedMemoriesResponse { items: vec![] }),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(self.default_pinned_response.clone()),
        }
    }

    // Design note: pin_memory has a special default behavior.
    // Unlike other single-value methods that return errors when no response is configured,
    // this method constructs a synthetic PinnedMemory from the request parameters.
    // This is intentional to simplify common test scenarios where the exact response
    // content is less important than verifying the call was made.
    async fn pin_memory(
        &self,
        memory_id: i64,
        reason: Option<String>,
    ) -> Result<PinnedMemory, LoomClientError> {
        self.record_call(MockCall::PinMemory { memory_id, reason: reason.clone() });
        match self.pop_response() {
            Some(Ok(MockResponse::PinnedMemory(p))) => Ok(p),
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(PinnedMemory {
                fragment: MemoryFragment {
                    id: memory_id,
                    content: String::new(),
                    timestamp: time::OffsetDateTime::now_utc(),
                    kind: loom_common::types::MemoryKind::Action,
                },
                reason,
                pinned_at: time::OffsetDateTime::now_utc(),
            }),
        }
    }

    async fn unpin_memory(&self, memory_id: i64) -> Result<(), LoomClientError> {
        self.record_call(MockCall::UnpinMemory { memory_id });
        match self.pop_response() {
            Some(Err(e)) => Err(LoomClientError::ApiError(e)),
            _ => Ok(()),
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockLoomClient>();
    }

    #[tokio::test]
    async fn test_mock_health_check() {
        let mut mock = MockLoomClient::new();
        mock.push_health_check(serde_json::json!({"status": "healthy"}));

        let result = mock.health_check().await.unwrap();
        assert_eq!(result["status"], "healthy");

        assert!(mock.was_called(|c| matches!(c, MockCall::HealthCheck)));
    }

    #[tokio::test]
    async fn test_mock_create_memory() {
        let mut mock = MockLoomClient::new();
        let fragment = MemoryFragment {
            id: 0,
            content: "test".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_common::types::MemoryKind::Action,
        };
        mock.push_memory(MemoryResponse { fragments: vec![fragment.clone()], total: 1 });

        let request = CreateMemoryRequest::single(fragment);
        let result = mock.create_memory(request).await.unwrap();

        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.total, 1);
        assert!(mock.was_called(|c| matches!(c, MockCall::CreateMemory { .. })));
    }

    #[tokio::test]
    async fn test_mock_get_memory() {
        let mut mock = MockLoomClient::new();
        let fragment = MemoryFragment {
            id: 42,
            content: "test content".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_common::types::MemoryKind::Action,
        };
        mock.push_memory(MemoryResponse { fragments: vec![fragment], total: 1 });

        let result = mock.get_memory(42).await.unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].id, 42);

        assert!(mock.was_called(|c| matches!(c, MockCall::GetMemory { id: 42 })));
    }

    #[tokio::test]
    async fn test_mock_error_response() {
        let mut mock = MockLoomClient::new();
        mock.push_error("Service unavailable");

        let result = mock.get_memory(1).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoomClientError::ApiError(_)));
    }

    #[tokio::test]
    async fn test_mock_call_count() {
        let mut mock = MockLoomClient::new();
        mock.push_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.push_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.push_memory(MemoryResponse { fragments: vec![], total: 0 });

        let _ = mock.get_memory(1).await;
        let _ = mock.get_memory(2).await;
        let _ = mock.get_memory(3).await;

        // Verify exact call sequence using PartialEq
        assert_eq!(
            mock.get_calls(),
            vec![
                MockCall::GetMemory { id: 1 },
                MockCall::GetMemory { id: 2 },
                MockCall::GetMemory { id: 3 }
            ]
        );
    }

    #[tokio::test]
    async fn test_mock_pinned_memories() {
        let mut mock = MockLoomClient::new();
        mock.push_pinned_memories(PinnedMemoriesResponse { items: vec![] });

        let result = mock.get_pinned_memories().await.unwrap();
        assert!(result.items.is_empty());

        assert!(mock.was_called(|c| matches!(c, MockCall::GetPinnedMemories)));
    }

    #[tokio::test]
    async fn test_mock_pin_memory() {
        let mut mock = MockLoomClient::new();
        let fragment = MemoryFragment {
            id: 100,
            content: "pinned content".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_common::types::MemoryKind::Action,
        };
        mock.push_pinned_memory(PinnedMemory {
            fragment,
            reason: Some("test reason".to_string()),
            pinned_at: time::OffsetDateTime::now_utc(),
        });

        let result = mock
            .pin_memory(100, Some("test reason".to_string()))
            .await
            .unwrap();
        assert_eq!(result.fragment.id, 100);
        assert_eq!(result.reason, Some("test reason".to_string()));
    }

    #[tokio::test]
    async fn test_mock_unpin_memory() {
        let mut mock = MockLoomClient::new();
        mock.push_empty();

        let result = mock.unpin_memory(100).await;
        assert!(result.is_ok());

        assert!(mock.was_called(|c| matches!(c, MockCall::UnpinMemory { memory_id: 100 })));
    }

    #[tokio::test]
    async fn test_mock_default_response() {
        let mut mock = MockLoomClient::new();
        let fragment = MemoryFragment {
            id: 1,
            content: "default".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_common::types::MemoryKind::Action,
        };
        mock.set_default_memory(MemoryResponse { fragments: vec![fragment], total: 1 });

        // No response pushed, should use default
        let result = mock.get_recent_memories(10).await.unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].id, 1);
    }

    #[tokio::test]
    async fn test_mock_clear_calls() {
        let mut mock = MockLoomClient::new();
        mock.push_memory(MemoryResponse { fragments: vec![], total: 0 });

        let _ = mock.get_memory(1).await;
        assert_eq!(mock.get_calls().len(), 1);

        mock.clear_calls();
        assert!(mock.get_calls().is_empty());
    }

    #[tokio::test]
    async fn test_mock_sequential_responses() {
        let mut mock = MockLoomClient::new();

        // Push multiple responses in sequence
        mock.push_memory(MemoryResponse {
            fragments: vec![MemoryFragment {
                id: 1,
                content: "first".to_string(),
                timestamp: time::OffsetDateTime::now_utc(),
                kind: loom_common::types::MemoryKind::Action,
            }],
            total: 1,
        });
        mock.push_memory(MemoryResponse {
            fragments: vec![MemoryFragment {
                id: 2,
                content: "second".to_string(),
                timestamp: time::OffsetDateTime::now_utc(),
                kind: loom_common::types::MemoryKind::Action,
            }],
            total: 1,
        });
        mock.push_error("third call fails");

        // First call returns first response
        let result1 = mock.get_memory(1).await.unwrap();
        assert_eq!(result1.fragments[0].id, 1);

        // Second call returns second response
        let result2 = mock.get_memory(2).await.unwrap();
        assert_eq!(result2.fragments[0].id, 2);

        // Third call returns error
        let result3 = mock.get_memory(3).await;
        assert!(result3.is_err());
    }

    #[tokio::test]
    async fn test_mock_with_trait_object() {
        // Test that mock works through Arc<dyn LoomClientTrait>
        let mut mock = MockLoomClient::new();
        mock.push_health_check(serde_json::json!({"status": "ok"}));

        let client: Arc<dyn LoomClientTrait> = Arc::new(mock);

        let result = client.health_check().await.unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[tokio::test]
    async fn test_mock_timeline_query() {
        let mut mock = MockLoomClient::new();
        let fragment = MemoryFragment {
            id: 1,
            content: "timeline event".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_common::types::MemoryKind::Action,
        };
        mock.push_memory(MemoryResponse { fragments: vec![fragment], total: 1 });

        let result = mock
            .get_timeline_memory("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z", None, None)
            .await
            .unwrap();
        assert_eq!(result.fragments.len(), 1);

        // Verify call was recorded with correct parameters
        assert!(mock.was_called(|c| matches!(c, MockCall::GetTimelineMemory { from, to, .. } if from == "2024-01-01T00:00:00Z" && to == "2024-12-31T23:59:59Z")));
    }

    #[tokio::test]
    async fn test_mock_delete_memory() {
        let mut mock = MockLoomClient::new();
        mock.push_empty();

        let result = mock.delete_memory(42).await;
        assert!(result.is_ok());

        assert!(mock.was_called(|c| matches!(c, MockCall::DeleteMemory { id: 42 })));
    }

    #[tokio::test]
    async fn test_mock_chain_configuration() {
        // Test builder-style chain configuration
        let mut mock = MockLoomClient::new();
        mock.push_health_check(serde_json::json!({"status": "healthy"}))
            .push_memory(MemoryResponse { fragments: vec![], total: 0 })
            .push_error("oops");

        let result1 = mock.health_check().await.unwrap();
        assert_eq!(result1["status"], "healthy");

        let result2 = mock.get_memory(1).await.unwrap();
        assert!(result2.fragments.is_empty());

        let result3 = mock.get_memory(2).await;
        assert!(result3.is_err());
    }
}

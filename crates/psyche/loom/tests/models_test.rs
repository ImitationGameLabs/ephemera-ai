use loom::memory::models::{
    ApiResponse, CreateMemoryRequest, MemoryQuery, MemoryResponse, SearchMemoryRequest, TimeRange,
};
use loom::memory::types::{MemoryFragment, MemoryKind};
use time::OffsetDateTime;

fn test_fragment(content: &str, kind: MemoryKind) -> MemoryFragment {
    MemoryFragment {
        id: 0,
        content: content.to_string(),
        timestamp: OffsetDateTime::now_utc(),
        kind,
    }
}

#[test]
fn test_memory_response_single() {
    let fragment = test_fragment("single content", MemoryKind::Event);
    let response = MemoryResponse::single(fragment);

    assert_eq!(response.total, 1);
    assert_eq!(response.len(), 1);
    assert!(!response.is_empty());
    assert!(response.first().is_some());
    assert_eq!(response.first().unwrap().content, "single content");
}

#[test]
fn test_memory_response_multiple() {
    let fragments = vec![
        test_fragment("first", MemoryKind::Event),
        test_fragment("second", MemoryKind::Thought),
        test_fragment("third", MemoryKind::Action),
    ];

    let response = MemoryResponse::multiple(fragments);

    assert_eq!(response.total, 3);
    assert_eq!(response.len(), 3);
    assert!(!response.is_empty());
    assert!(response.first().is_some());
    assert_eq!(response.first().unwrap().content, "first");
}

#[test]
fn test_memory_response_empty() {
    let response = MemoryResponse::multiple(vec![]);

    assert_eq!(response.total, 0);
    assert_eq!(response.len(), 0);
    assert!(response.is_empty());
    assert!(response.first().is_none());
}

#[test]
fn test_memory_response_first_and_is_empty() {
    let fragment = test_fragment("test", MemoryKind::Event);
    let response = MemoryResponse::single(fragment);
    assert!(response.first().is_some());
    assert_eq!(response.first().unwrap().content, "test");
    assert!(!response.is_empty());

    let fragments =
        vec![test_fragment("a", MemoryKind::Thought), test_fragment("b", MemoryKind::Action)];
    let response = MemoryResponse::multiple(fragments);
    assert_eq!(response.first().unwrap().content, "a");
}

#[test]
fn test_create_memory_request_single() {
    let fragment = test_fragment("single", MemoryKind::Event);
    let request = CreateMemoryRequest::single(fragment);

    assert_eq!(request.fragments.len(), 1);
    assert_eq!(request.fragments[0].content, "single");
}

#[test]
fn test_create_memory_request_multiple() {
    let fragments =
        vec![test_fragment("a", MemoryKind::Event), test_fragment("b", MemoryKind::Thought)];
    let request = CreateMemoryRequest::multiple(fragments);

    assert_eq!(request.fragments.len(), 2);
}

#[test]
fn test_api_response_success() {
    let response: ApiResponse<String> = ApiResponse::success("test data".to_string());

    assert!(response.success);
    assert_eq!(response.data, Some("test data".to_string()));
    assert_eq!(response.error, None);
}

#[test]
fn test_api_response_error() {
    let response: ApiResponse<String> = ApiResponse::error("something went wrong");

    assert!(!response.success);
    assert_eq!(response.data, None);
    assert_eq!(response.error, Some("something went wrong".to_string()));
}

#[test]
fn test_search_request_to_query_with_time_range() {
    let request = SearchMemoryRequest {
        keywords: "test search".to_string(),
        start_time: Some(1000),
        end_time: Some(2000),
    };

    let query: MemoryQuery = request.into();

    assert_eq!(query.keywords, "test search");
    assert!(query.time_range.is_some());
    let range = query.time_range.unwrap();
    assert_eq!(range.start, 1000);
    assert_eq!(range.end, 2000);
}

#[test]
fn test_search_request_to_query_without_time_range() {
    let request = SearchMemoryRequest {
        keywords: "test search".to_string(),
        start_time: None,
        end_time: None,
    };

    let query: MemoryQuery = request.into();

    assert_eq!(query.keywords, "test search");
    assert!(query.time_range.is_none());
}

#[test]
fn test_search_request_to_query_partial_time_range() {
    let request = SearchMemoryRequest {
        keywords: "test".to_string(),
        start_time: Some(1000),
        end_time: None,
    };
    let query: MemoryQuery = request.into();
    assert!(query.time_range.is_none());

    let request = SearchMemoryRequest {
        keywords: "test".to_string(),
        start_time: None,
        end_time: Some(2000),
    };
    let query: MemoryQuery = request.into();
    assert!(query.time_range.is_none());
}

#[test]
fn test_time_range_creation() {
    let range = TimeRange { start: 100, end: 200 };

    assert_eq!(range.start, 100);
    assert_eq!(range.end, 200);
}

#[test]
fn test_memory_query_creation() {
    let query = MemoryQuery {
        keywords: "search terms".to_string(),
        time_range: Some(TimeRange { start: 0, end: 1000 }),
    };

    assert_eq!(query.keywords, "search terms");
    assert!(query.time_range.is_some());
}

#[test]
fn test_memory_response_serialization() {
    let fragment = test_fragment("serialize me", MemoryKind::Event);
    let response = MemoryResponse::single(fragment);

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("serialize me"));
    assert!(json.contains("\"total\":1"));
}

#[test]
fn test_memory_response_deserialization() {
    let json = r#"{"fragments":[],"total":0}"#;
    let response: MemoryResponse = serde_json::from_str(json).unwrap();

    assert!(response.is_empty());
    assert_eq!(response.total, 0);
}

#[test]
fn test_api_response_serialization() {
    let response: ApiResponse<String> = ApiResponse::success("data".to_string());
    let json = serde_json::to_string(&response).unwrap();

    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"data\":\"data\""));
}

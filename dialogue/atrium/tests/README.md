# Dialogue Atrium API Integration Tests

This directory contains integration tests for the Dialogue Atrium API endpoints.

## Running Tests

### Prerequisites

1. **Ensure MySQL and other services are running:**
   ```bash
   docker compose up -d
   ```

2. **Start the Dialogue Atrium server:**
   ```bash
   cargo run --bin dialogue-atrium
   ```

### Running Tests

From the project root directory:
```bash
cargo test -p dialogue-atrium --test api_integration -- --nocapture
```

### Running Specific Tests

Run specific test categories:
```bash
# User lifecycle tests
cargo test -p dialogue-atrium test_user_lifecycle -- --nocapture

# Message lifecycle tests
cargo test -p dialogue-atrium test_message_lifecycle -- --nocapture

# Message incremental fetching tests
cargo test -p dialogue-atrium test_message_incremental_fetching -- --nocapture

# Error handling tests
cargo test -p dialogue-atrium test_error_handling -- --nocapture
```

## Troubleshooting

### Server Not Running
If you get "server is not running" error:
1. Start the server: `cargo run --bin dialogue-atrium`
2. Wait a few seconds for the server to fully start
3. Try running the tests again

### Database Connection Issues
If tests fail due to database connection:
1. Ensure Docker services are running: `docker compose up -d`
2. Check environment variables in `.env` file
3. Verify database is accessible

## Adding New Tests

To add new integration tests:

1. Add new test functions to `api_integration.rs`
2. Follow the existing naming pattern: `test_<feature>_<scenario>`
3. Use the `TestClient` struct for HTTP requests
4. Include proper assertions and error handling

Example new test:
```rust
#[tokio::test]
async fn test_new_feature() {
    let client = TestClient::new();

    // Arrange - setup test data
    // Act - perform API call
    // Assert - verify response
    assert_eq!(response.status(), expected_status);
}
```

## Testing since_id Functionality

The `since_id` parameter enables incremental message fetching for efficient polling. Here are key test scenarios:

### Test Scenarios

1. **Basic Incremental Fetching:**
   - Create multiple messages
   - Get messages with `since_id` parameter
   - Verify only messages with greater IDs are returned
   - Verify messages are ordered by ID (ascending)

2. **Parameter Override Behavior:**
   - Test that `since_id` ignores `sender` parameter
   - Test that `since_id` ignores `offset` parameter
   - Verify `limit` parameter still works with `since_id`

3. **Edge Cases:**
   - Test with non-existent `since_id` value
   - Test with `since_id` of 1 (first message)
   - Test with `since_id` larger than latest message ID
   - Test with `since_id` combined with `limit`

### Example Test

```rust
#[tokio::test]
async fn test_message_incremental_fetching() {
    let client = TestClient::new();

    // Arrange - create test messages
    let msg1 = client.create_message("Hello from user1", "user1", "pass1").await;
    let msg2 = client.create_message("Hello from user2", "user2", "pass2").await;
    let msg3 = client.create_message("Another message", "user1", "pass1").await;

    // Act - fetch messages since second message
    let response = client.get_messages_with_since_id(msg2.id, Some(10)).await;

    // Assert - should only return messages with ID > msg2.id
    assert_eq!(response.status(), 200);
    let messages: Messages = response.json().await;
    assert_eq!(messages.messages.len(), 1);
    assert_eq!(messages.messages[0].id, msg3.id);
}
```
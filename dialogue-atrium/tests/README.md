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
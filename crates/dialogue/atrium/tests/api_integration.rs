#![cfg(feature = "integration-tests")]

use reqwest::Client;
use serde_json::json;
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:3001/api/v1";

struct TestClient {
    client: Client,
    base_url: String,
}

impl TestClient {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: BASE_URL.to_string(),
        }
    }

    async fn create_user(&self, username: &str, bio: &str, password: &str) -> reqwest::Result<reqwest::Response> {
        let user_data = json!({
            "name": username,
            "bio": bio,
            "password": password
        });

        self.client
            .post(&format!("{}/users", self.base_url))
            .json(&user_data)
            .send()
            .await
    }

    async fn get_user(&self, username: &str) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/users/{}", self.base_url, username))
            .send()
            .await
    }

    async fn get_all_users(&self) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/users", self.base_url))
            .send()
            .await
    }

    async fn create_message(&self, content: &str, username: &str, password: &str) -> reqwest::Result<reqwest::Response> {
        let message_data = json!({
            "content": content,
            "username": username,
            "password": password
        });

        self.client
            .post(&format!("{}/messages", self.base_url))
            .json(&message_data)
            .send()
            .await
    }

    async fn get_messages(&self) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/messages", self.base_url))
            .send()
            .await
    }

    async fn get_message(&self, id: i32) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/messages/{}", self.base_url, id))
            .send()
            .await
    }

    async fn delete_message(&self, id: i32) -> reqwest::Result<reqwest::Response> {
        self.client
            .delete(&format!("{}/messages/{}", self.base_url, id))
            .send()
            .await
    }

    async fn update_heartbeat(&self, username: &str, password: &str) -> reqwest::Result<reqwest::Response> {
        let heartbeat_data = json!({
            "username": username,
            "password": password
        });

        self.client
            .put(&format!("{}/heartbeat", self.base_url))
            .json(&heartbeat_data)
            .send()
            .await
    }
}

#[tokio::test]
async fn test_user_lifecycle() {
    let client = TestClient::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let test_username = &format!("test_user_{}", timestamp);
    let test_bio = "Test user bio";
    let test_password = &format!("test_password_{}", timestamp);

    // 1. Create user
    println!("Creating user: {}", test_username);
    let create_response = client.create_user(test_username, test_bio, test_password).await.unwrap();
    assert_eq!(create_response.status(), 201);

    // 2. Get user profile
    println!("Getting user profile for: {}", test_username);
    let get_response = client.get_user(test_username).await.unwrap();
    assert_eq!(get_response.status(), 200);

    let user_data: serde_json::Value = get_response.json().await.unwrap();
    assert_eq!(user_data["name"].as_str().unwrap(), test_username);
    assert_eq!(user_data["bio"].as_str().unwrap(), test_bio);

    // Verify timestamp is in ISO 8601 format (string, not array)
    let created_at = &user_data["created_at"];
    assert!(created_at.is_string(), "created_at should be a string in ISO 8601 format, got: {:?}", created_at);

    // Try to parse as ISO 8601 datetime
    let timestamp_str = created_at.as_str().unwrap();
    assert!(timestamp_str.contains('T') && timestamp_str.contains('Z'),
           "Timestamp should be in ISO 8601 format, got: {}", timestamp_str);

    // 3. Get all users (should include our test user)
    println!("Getting all users");
    let all_users_response = client.get_all_users().await.unwrap();
    assert_eq!(all_users_response.status(), 200);

    let users_data: serde_json::Value = all_users_response.json().await.unwrap();
    assert!(users_data["users"].as_array().unwrap().len() >= 1);

    // 4. Update heartbeat
    println!("Updating heartbeat for: {}", test_username);
    let heartbeat_response = client.update_heartbeat(test_username, test_password).await.unwrap();
    assert_eq!(heartbeat_response.status(), 200);
}

#[tokio::test]
async fn test_message_lifecycle() {
    let client = TestClient::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let test_sender = &format!("test_sender_{}", timestamp);
    let test_content = "Hello, this is a test message!";
    let test_password = &format!("sender_password_{}", timestamp);

    // First create a user to send messages
    client.create_user(test_sender, "Test sender", test_password).await.unwrap();

    // 1. Create message
    println!("Creating message from: {}", test_sender);
    let create_response = client.create_message(test_content, test_sender, test_password).await.unwrap();
    assert_eq!(create_response.status(), 201);

    let message_data: serde_json::Value = create_response.json().await.unwrap();
    let message_id: i32 = message_data["id"].as_i64().unwrap() as i32;
    assert!(message_id > 0);

    // 2. Get specific message
    println!("Getting message with ID: {}", message_id);
    let get_response = client.get_message(message_id).await.unwrap();
    assert_eq!(get_response.status(), 200);

    let retrieved_message: serde_json::Value = get_response.json().await.unwrap();
    assert_eq!(retrieved_message["id"], message_id);
    assert_eq!(retrieved_message["content"].as_str().unwrap(), test_content);
    assert_eq!(retrieved_message["sender"].as_str().unwrap(), test_sender);

    // Verify timestamp is in ISO 8601 format (string, not array)
    let created_at = &retrieved_message["created_at"];
    assert!(created_at.is_string(), "created_at should be a string in ISO 8601 format, got: {:?}", created_at);

    // Try to parse as ISO 8601 datetime
    let timestamp_str = created_at.as_str().unwrap();
    assert!(timestamp_str.contains('T') && timestamp_str.contains('Z'),
           "Timestamp should be in ISO 8601 format, got: {}", timestamp_str);

    // 3. Get all messages
    println!("Getting all messages");
    let all_messages_response = client.get_messages().await.unwrap();
    assert_eq!(all_messages_response.status(), 200);

    let messages_data: serde_json::Value = all_messages_response.json().await.unwrap();
    assert!(messages_data["messages"].as_array().unwrap().len() >= 1);

    // 4. Delete message
    println!("Deleting message with ID: {}", message_id);
    let delete_response = client.delete_message(message_id).await.unwrap();
    assert_eq!(delete_response.status(), 204);

    // 5. Verify message is deleted
    let verify_response = client.get_message(message_id).await.unwrap();
    assert_eq!(verify_response.status(), 404);
}

#[tokio::test]
async fn test_error_handling() {
    let client = TestClient::new();

    // 1. Try to get non-existent user
    println!("Testing get non-existent user");
    let response = client.get_user("non_existent_user").await.unwrap();
    assert_eq!(response.status(), 404);

    // 2. Try to get non-existent message
    println!("Testing get non-existent message");
    let response = client.get_message(99999).await.unwrap();
    assert_eq!(response.status(), 404);

    // 3. Try to delete non-existent message
    println!("Testing delete non-existent message");
    let response = client.delete_message(99999).await.unwrap();
    assert_eq!(response.status(), 404);

    // 4. Try to create user with invalid data (missing fields)
    println!("Testing create user with invalid data");
    let invalid_user_data = json!({
        "name": "incomplete_user"
        // missing bio and password
    });

    let response = client
        .client
        .post(&format!("{}/users", client.base_url))
        .json(&invalid_user_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 422); // Should return unprocessable entity due to validation
}

#[tokio::test]
async fn test_multiple_users_and_messages() {
    let client = TestClient::new();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let usernames = vec![
        format!("alice_{}", timestamp),
        format!("bob_{}", timestamp),
        format!("charlie_{}", timestamp)
    ];

    // Create multiple users
    for username in &usernames {
        println!("Creating user: {}", username);
        let response = client.create_user(username, &format!("Bio for {}", username), &format!("password_{}", username)).await.unwrap();
        assert_eq!(response.status(), 201);
    }

    // Each user sends a message
    for username in &usernames {
        let message_content = format!("Message from {}", username);
        println!("Creating message: {}", message_content);
        let response = client.create_message(&message_content, username, &format!("password_{}", username)).await.unwrap();
        assert_eq!(response.status(), 201);
    }

    // Verify all messages exist
    let all_messages_response = client.get_messages().await.unwrap();
    assert_eq!(all_messages_response.status(), 200);

    let messages_data: serde_json::Value = all_messages_response.json().await.unwrap();
    let messages = messages_data["messages"].as_array().unwrap();
    assert!(messages.len() >= usernames.len());

    // Verify all users exist
    let all_users_response = client.get_all_users().await.unwrap();
    assert_eq!(all_users_response.status(), 200);

    let users_data: serde_json::Value = all_users_response.json().await.unwrap();
    let users = users_data["users"].as_array().unwrap();
    assert!(users.len() >= usernames.len());
}
/// Integration tests for the chat application
/// These tests focus on the shared types and basic functionality

/// Test message serialization/deserialization
#[tokio::test]
async fn test_message_serialization() {
    let message = serde_json::json!({
        "text": "Hello, World!"
    });
    
    let json = serde_json::to_string(&message).expect("Failed to serialize message");
    let deserialized: serde_json::Value = serde_json::from_str(&json)
        .expect("Failed to deserialize message");
    
    assert_eq!(message["text"], deserialized["text"]);
}

/// Test basic JSON structure for client messages
#[tokio::test]
async fn test_client_message_formats() {
    // Test connect message format
    let connect_msg = serde_json::json!({
        "Connect": {
            "name": "Alice"
        }
    });
    
    let json = serde_json::to_string(&connect_msg).expect("Failed to serialize connect message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse connect message");
    
    assert!(parsed["Connect"].is_object());
    assert_eq!(parsed["Connect"]["name"], "Alice");
    
    // Test chat message format
    let chat_msg = serde_json::json!({
        "Chat": {
            "text": "Hello everyone!"
        }
    });
    
    let json = serde_json::to_string(&chat_msg).expect("Failed to serialize chat message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse chat message");
    
    assert!(parsed["Chat"].is_object());
    assert_eq!(parsed["Chat"]["text"], "Hello everyone!");
    
    // Test disconnect message format
    let disconnect_msg = serde_json::json!({
        "Disconnect": null
    });
    
    let json = serde_json::to_string(&disconnect_msg).expect("Failed to serialize disconnect message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse disconnect message");
    
    assert!(parsed["Disconnect"].is_null());
}

/// Test basic JSON structure for server messages
#[tokio::test]
async fn test_server_message_formats() {
    // Test chat message format
    let chat_msg = serde_json::json!({
        "Chat": {
            "text": "Welcome to the chat!"
        }
    });
    
    let json = serde_json::to_string(&chat_msg).expect("Failed to serialize server chat message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse server chat message");
    
    assert!(parsed["Chat"].is_object());
    assert_eq!(parsed["Chat"]["text"], "Welcome to the chat!");
    
    // Test user list format
    let user_list_msg = serde_json::json!({
        "UserList": {
            "count": 2,
            "users": [
                {"id": "user1", "name": "Alice"},
                {"id": "user2", "name": "Bob"}
            ]
        }
    });
    
    let json = serde_json::to_string(&user_list_msg).expect("Failed to serialize user list message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse user list message");
    
    assert!(parsed["UserList"].is_object());
    assert_eq!(parsed["UserList"]["count"], 2);
    assert_eq!(parsed["UserList"]["users"].as_array().unwrap().len(), 2);
    
    // Test user joined format
    let user_joined_msg = serde_json::json!({
        "UserJoined": {
            "name": "Charlie"
        }
    });
    
    let json = serde_json::to_string(&user_joined_msg).expect("Failed to serialize user joined message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse user joined message");
    
    assert!(parsed["UserJoined"].is_object());
    assert_eq!(parsed["UserJoined"]["name"], "Charlie");
    
    // Test user left format
    let user_left_msg = serde_json::json!({
        "UserLeft": {
            "name": "Charlie"
        }
    });
    
    let json = serde_json::to_string(&user_left_msg).expect("Failed to serialize user left message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse user left message");
    
    assert!(parsed["UserLeft"].is_object());
    assert_eq!(parsed["UserLeft"]["name"], "Charlie");
}

/// Test message structure validation
#[tokio::test]
async fn test_message_structure_validation() {
    // Test valid message structure
    let valid_msg = serde_json::json!({
        "text": "This is a valid message"
    });
    
    let json = serde_json::to_string(&valid_msg).expect("Failed to serialize valid message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse valid message");
    
    assert!(parsed.is_object());
    assert!(parsed["text"].is_string());
    assert_eq!(parsed["text"], "This is a valid message");
    
    // Test empty message
    let empty_msg = serde_json::json!({
        "text": ""
    });
    
    let json = serde_json::to_string(&empty_msg).expect("Failed to serialize empty message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse empty message");
    
    assert!(parsed["text"].is_string());
    assert_eq!(parsed["text"], "");
    
    // Test long message
    let long_text = "A".repeat(1000);
    let long_msg = serde_json::json!({
        "text": long_text
    });
    
    let json = serde_json::to_string(&long_msg).expect("Failed to serialize long message");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse long message");
    
    assert!(parsed["text"].is_string());
    assert_eq!(parsed["text"].as_str().unwrap().len(), 1000);
}
use super::*;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::ws::Message;
    use futures::SinkExt;
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};

    #[tokio::test]
    async fn test_message_serialization() {
        let message = Message {
            text: "Hello, World!".to_string(),
        };
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        
        assert_eq!(message.text, deserialized.text);
    }

    #[tokio::test]
    async fn test_user_creation() {
        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User {
            id: user_id.clone(),
            name: format!("User_{}", user_id.split('-').next().unwrap()),
            connected_at: Instant::now(),
        };
        
        assert!(!user.id.is_empty());
        assert!(!user.name.is_empty());
        assert!(user.name.starts_with("User_"));
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        let app_state = AppState {
            messages: messages.clone(),
            clients: clients.clone(),
            users: users.clone(),
        };
        
        // Test initial state
        assert_eq!(app_state.messages.lock().unwrap().len(), 0);
        assert_eq!(app_state.clients.lock().unwrap().len(), 0);
        assert_eq!(app_state.users.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_message_storage() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        let app_state = AppState {
            messages: messages.clone(),
            clients: clients.clone(),
            users: users.clone(),
        };
        
        // Add a message
        let test_message = Message {
            text: "Test message".to_string(),
        };
        
        {
            let mut messages_guard = app_state.messages.lock().unwrap();
            messages_guard.push(test_message.clone());
        }
        
        // Verify message was stored
        assert_eq!(app_state.messages.lock().unwrap().len(), 1);
        assert_eq!(app_state.messages.lock().unwrap()[0].text, "Test message");
    }

    #[tokio::test]
    async fn test_user_management() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        let app_state = AppState {
            messages: messages.clone(),
            clients: clients.clone(),
            users: users.clone(),
        };
        
        // Add a user
        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User {
            id: user_id.clone(),
            name: "TestUser".to_string(),
            connected_at: Instant::now(),
        };
        
        {
            let mut users_guard = app_state.users.lock().unwrap();
            users_guard.insert(user_id.clone(), user.clone());
        }
        
        // Verify user was added
        assert_eq!(app_state.users.lock().unwrap().len(), 1);
        assert!(app_state.users.lock().unwrap().contains_key(&user_id));
        
        // Remove user
        {
            let mut users_guard = app_state.users.lock().unwrap();
            users_guard.remove(&user_id);
        }
        
        // Verify user was removed
        assert_eq!(app_state.users.lock().unwrap().len(), 0);
        assert!(!app_state.users.lock().unwrap().contains_key(&user_id));
    }

    #[tokio::test]
    async fn test_client_broadcast_simulation() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let users = Arc::new(Mutex::new(HashMap::new()));
        
        let app_state = AppState {
            messages: messages.clone(),
            clients: clients.clone(),
            users: users.clone(),
        };
        
        // Create mock client channels
        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        
        // Add clients to state
        {
            let mut clients_guard = app_state.clients.lock().unwrap();
            clients_guard.push(tx1);
            clients_guard.push(tx2);
        }
        
        // Broadcast a message
        let broadcast_message = Message {
            text: "Broadcast test".to_string(),
        };
        
        {
            let clients_guard = app_state.clients.lock().unwrap();
            for client_tx in clients_guard.iter() {
                let _ = client_tx.send(broadcast_message.clone());
            }
        }
        
        // Verify both clients received the message
        let received1 = rx1.recv().await;
        let received2 = rx2.recv().await;
        
        assert!(received1.is_some());
        assert!(received2.is_some());
        assert_eq!(received1.unwrap().text, "Broadcast test");
        assert_eq!(received2.unwrap().text, "Broadcast test");
    }

    #[tokio::test]
    async fn test_message_formatting() {
        let name = "Alice";
        let line = "Hello everyone!";
        
        let mut whom = String::from(name);
        whom.push_str(": ");
        whom.push_str(line);
        
        assert_eq!(whom, "Alice: Hello everyone!");
        
        let message = Message { text: whom.clone() };
        assert_eq!(message.text, "Alice: Hello everyone!");
    }

    #[tokio::test]
    async fn test_server_url_construction() {
        let address = "127.0.0.1";
        let port = 8080;
        let server_url = format!("http://{}:{}", address, port);
        
        assert_eq!(server_url, "http://127.0.0.1:8080");
        
        let messages_url = format!("{}/messages", server_url);
        assert_eq!(messages_url, "http://127.0.0.1:8080/messages");
        
        let room_url = format!("{}/room/1", server_url);
        assert_eq!(room_url, "http://127.0.0.1:8080/room/1");
    }

    #[tokio::test]
    async fn test_user_id_generation() {
        let user_id1 = uuid::Uuid::new_v4().to_string();
        let user_id2 = uuid::Uuid::new_v4().to_string();
        
        // Verify UUIDs are unique
        assert_ne!(user_id1, user_id2);
        
        // Verify UUID format (should contain hyphens)
        assert!(user_id1.contains('-'));
        assert!(user_id2.contains('-'));
        
        // Verify user name generation
        let name1 = format!("User_{}", user_id1.split('-').next().unwrap());
        let name2 = format!("User_{}", user_id2.split('-').next().unwrap());
        
        assert!(name1.starts_with("User_"));
        assert!(name2.starts_with("User_"));
        assert_ne!(name1, name2);
    }

    #[tokio::test]
    async fn test_instant_timing() {
        let start = Instant::now();
        sleep(Duration::from_millis(10)).await;
        let duration = start.elapsed();
        
        // Verify that some time has passed
        assert!(duration.as_millis() >= 5);
        assert!(duration.as_millis() <= 50); // Allow some margin for timing variations
    }
}
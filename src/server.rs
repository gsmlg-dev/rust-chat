use axum::{
    Json, Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::shared::{ChatError, ChatResult, ClientMessage, Message, ServerMessage, User, UserList};

/// Maximum number of messages to keep in memory
const MAX_MESSAGES: usize = 1000;

/// Represents the shared application state for the chat server.
///
/// This struct contains all the data that needs to be shared across
/// different async tasks and WebSocket connections.
#[derive(Clone)]
pub struct AppState {
    /// Stores the chat message history
    pub messages: Arc<Mutex<Vec<Message>>>,
    /// List of active WebSocket client connections
    pub clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<Message>>>>,
    /// Mapping of user IDs to user information
    pub users: Arc<Mutex<HashMap<String, User>>>,
}

/// Starts the chat server with the specified configuration.
///
/// # Arguments
///
/// * `address` - The IP address to bind the server to (e.g., "127.0.0.1")
/// * `port` - The port number to listen on (e.g., 12345)
/// * `tui` - Whether to enable the terminal user interface
///
/// # Returns
///
/// Returns `Ok(())` if the server starts successfully, or an error if binding fails.
///
/// # Examples
///
/// ```rust
/// // Start server on localhost:12345 without TUI
/// run_server("127.0.0.1", 12345, false).await?;
/// ```
pub async fn run_server(address: &str, port: u16, tui: bool) -> ChatResult<()> {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let clients = Arc::new(Mutex::new(Vec::new()));
    let users = Arc::new(Mutex::new(HashMap::new()));
    let app_state = AppState {
        messages,
        clients,
        users,
    };

    let addr = format!("{}:{}", address, port);
    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");

    if tui {
        println!("Chat server running on http://{} with TUI", socket_addr);
        run_tui_server(app_state.clone(), socket_addr).await?;
    } else {
        println!("Chat server running on http://{}", socket_addr);
        let app = Router::new()
            .route("/room/1", get(handle_websocket))
            .route("/room/1", post(handle_post))
            .route("/messages", get(handle_get))
            .with_state(app_state);

        let listener = tokio::net::TcpListener::bind(socket_addr)
            .await
            .map_err(|e| {
                ChatError::NetworkError(format!("Failed to bind to {}: {}", socket_addr, e))
            })?;

        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("Server error: {}", e);
            return Err(ChatError::NetworkError(format!(
                "Server runtime error: {}",
                e
            )));
        }
    }

    Ok(())
}

/// Handles WebSocket upgrade requests for the chat endpoint.
///
/// This function is called when a client attempts to upgrade their HTTP
/// connection to a WebSocket connection for real-time chat.
///
/// # Arguments
///
/// * `ws` - The WebSocket upgrade request from axum
/// * `state` - The shared application state
///
/// # Returns
///
/// Returns a response that upgrades the connection to WebSocket.
async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handles the actual WebSocket connection after upgrade.
///
/// This function manages the bidirectional communication with a connected
/// client, processing incoming messages and broadcasting outgoing messages.
///
/// # Arguments
///
/// * `socket` - The upgraded WebSocket connection
/// * `state` - The shared application state
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // First, wait for a connection message with the user's name
    let user_name = match receiver.next().await {
        Some(Ok(axum::extract::ws::Message::Text(text))) => {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Connect { name } => name,
                    _ => format!(
                        "User_{}",
                        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
                    ),
                }
            } else {
                // Fallback for old format - extract name from "Name: message" format
                if let Ok(msg) = serde_json::from_str::<Message>(&text) {
                    if let Some(colon_pos) = msg.text.find(':') {
                        msg.text[..colon_pos].to_string()
                    } else {
                        msg.text
                    }
                } else {
                    format!(
                        "User_{}",
                        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
                    )
                }
            }
        }
        _ => format!(
            "User_{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        ),
    };

    // Generate a unique user ID
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User::new(user_name.clone());

    // Add this client to list
    if let Err(e) = state.clients.lock() {
        eprintln!("Failed to acquire clients lock: {}", e);
        return;
    }
    state.clients.lock().unwrap().push(tx);

    // Add user to tracking
    {
        let mut users = state.users.lock().unwrap();
        users.insert(user_id.clone(), user.clone());
    }

    // Send existing messages to new client
    let messages_to_send = {
        let messages = state.messages.lock().unwrap();
        messages
            .iter()
            .map(|msg| msg.text.clone())
            .collect::<Vec<String>>()
    };

    for msg_text in messages_to_send {
        if sender
            .send(axum::extract::ws::Message::Text(msg_text.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Send user list to all clients
    broadcast_user_list(&state).await;

    // Broadcast user joined notification
    broadcast_user_joined(&state, &user_name).await;

    // Handle incoming messages from this client
    let state_clone = state.clone();
    let user_name_clone = user_name.clone();
    let recv_task = async {
        while let Some(msg) = receiver.next().await {
            if let Ok(axum::extract::ws::Message::Text(text)) = msg {
                // Try to parse as ClientMessage
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Chat { text: chat_text } => {
                            let message = Message::chat_message(&user_name_clone, &chat_text);

                            // Store message with limit
                            {
                                let mut messages = state_clone.messages.lock().unwrap();

                                // First check if we need to remove old messages
                                let needs_trimming = messages.len() >= MAX_MESSAGES;

                                // Add the new message
                                messages.push(message.clone());

                                // Remove oldest message if we exceeded the limit
                                if needs_trimming {
                                    messages.remove(0);
                                }
                            }

                            // Broadcast to all clients
                            let server_msg = ServerMessage::Chat { text: message.text };
                            broadcast_server_message(&state_clone, &server_msg).await;
                        }
                        ClientMessage::Disconnect => {
                            break;
                        }
                        ClientMessage::Connect { .. } => {
                            // Ignore duplicate connect messages
                        }
                    }
                } else {
                    // Fallback for old message format
                    let message = Message {
                        text: text.to_string(),
                    };

                    // Store message with limit
                    {
                        let mut messages = state_clone.messages.lock().unwrap();
                        messages.push(message.clone());

                        // Remove oldest messages if we exceed the limit
                        while messages.len() > MAX_MESSAGES {
                            messages.remove(0);
                        }
                    }

                    // Broadcast to all clients
                    let clients = state_clone.clients.lock().unwrap();
                    for client_tx in clients.iter() {
                        let _ = client_tx.send(message.clone());
                    }
                }
            }
        }
    };

    // Handle outgoing messages to this client
    let send_task = async {
        while let Some(msg) = rx.recv().await {
            if sender
                .send(axum::extract::ws::Message::Text(msg.text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    };

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }

    // Clean up user when disconnected
    {
        let mut users = state.users.lock().unwrap();
        users.remove(&user_id);
    }

    // Broadcast user left notification
    broadcast_user_left(&state, &user_name).await;
}

/// Handles GET requests to retrieve all chat messages.
///
/// This endpoint returns the complete message history as plain text,
/// with each message on a new line.
///
/// # Arguments
///
/// * `state` - The shared application state containing the messages
///
/// # Returns
///
/// Returns a response with status 200 OK containing the message history.
async fn handle_get(State(state): State<AppState>) -> impl IntoResponse {
    let messages = state.messages.lock().unwrap();
    let response: String = messages
        .iter()
        .map(|msg| format!("{}\n", msg.text))
        .collect();

    (StatusCode::OK, response)
}

/// Handles POST requests to add new chat messages.
///
/// This endpoint accepts JSON messages, stores them in the message history,
/// enforces the message limit, and broadcasts them to all connected WebSocket clients.
///
/// # Arguments
///
/// * `state` - The shared application state
/// * `message` - The message to add, extracted from the JSON request body
///
/// # Returns
///
/// Returns status 201 CREATED if the message is successfully processed.
async fn handle_post(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> impl IntoResponse {
    let mut messages = state.messages.lock().unwrap();
    messages.push(message.clone());

    // Remove oldest messages if we exceed the limit
    if messages.len() > MAX_MESSAGES {
        let drain_end = messages.len() - MAX_MESSAGES;
        messages.drain(0..drain_end);
    }

    // Broadcast to all WebSocket clients
    let clients = state.clients.lock().unwrap();
    for client_tx in clients.iter() {
        let _ = client_tx.send(message.clone());
    }

    StatusCode::CREATED
}

async fn run_tui_server(state: AppState, socket_addr: SocketAddr) -> ChatResult<()> {
    let listener = tokio::net::TcpListener::bind(socket_addr)
        .await
        .map_err(|e| {
            ChatError::NetworkError(format!("Failed to bind to {}: {}", socket_addr, e))
        })?;
    let state_clone = state.clone();

    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, {
            Router::new()
                .route("/room/1", get(handle_websocket))
                .route("/room/1", post(handle_post))
                .route("/messages", get(handle_get))
                .with_state(state_clone.clone())
        })
        .await
        .unwrap();
    });

    // Run TUI
    if let Err(e) = run_tui(state).await {
        eprintln!("TUI error: {}", e);
    }

    // Cancel server task
    server_handle.abort();

    Ok(())
}

async fn broadcast_user_list(state: &AppState) {
    let user_list = {
        let users = state.users.lock().unwrap();
        UserList::from_users(&users.values().cloned().collect::<Vec<_>>())
    };

    let server_msg = ServerMessage::UserList(user_list);
    broadcast_server_message(state, &server_msg).await;
}

async fn broadcast_user_joined(state: &AppState, user_name: &str) {
    let server_msg = ServerMessage::UserJoined {
        name: user_name.to_string(),
    };
    broadcast_server_message(state, &server_msg).await;
}

async fn broadcast_user_left(state: &AppState, user_name: &str) {
    let server_msg = ServerMessage::UserLeft {
        name: user_name.to_string(),
    };
    broadcast_server_message(state, &server_msg).await;
}

async fn broadcast_server_message(state: &AppState, server_msg: &ServerMessage) {
    let json = serde_json::to_string(server_msg).expect("Failed to serialize server message");
    let message = Message { text: json };

    let clients = state.clients.lock().unwrap();
    for client_tx in clients.iter() {
        let _ = client_tx.send(message.clone());
    }
}

async fn run_tui(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    println!("TUI Mode - Press Ctrl+C to exit");

    loop {
        // Clear screen and print status
        print!("\x1B[2J\x1B[H"); // Clear screen and move cursor to top

        let users_count = {
            let users_guard = state.users.lock().unwrap();
            users_guard.len()
        };
        let users_list = {
            let users_guard = state.users.lock().unwrap();
            users_guard.values().cloned().collect::<Vec<_>>()
        };

        println!("┌─────────────────────────────────────────┐");
        println!("│ Chat Server TUI                          │");
        println!("├─────────────────────────────────────────┤");
        println!("│ Connected Users: {}                     │", users_count);
        println!("├─────────────────────────────────────────┤");

        if users_list.is_empty() {
            println!("│ No users connected                      │");
        } else {
            for (i, user) in users_list.iter().enumerate() {
                let duration = user.connected_at.elapsed();
                if i < 10 {
                    // Limit display to 10 users
                    println!(
                        "│ {} ({}s ago)                       │",
                        user.name.chars().take(30).collect::<String>(),
                        duration.as_secs()
                    );
                }
            }
            if users_list.len() > 10 {
                println!(
                    "│ ... and {} more users                  │",
                    users_list.len() - 10
                );
            }
        }

        println!("├─────────────────────────────────────────┤");
        println!("│ Press Ctrl+C to quit                    │");
        println!("└─────────────────────────────────────────┘");

        // Sleep for 1 second before refreshing
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

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

    #[tokio::test]
    async fn test_server_start_and_stop() {
        let address = "127.0.0.1";
        let port = 12346; // Use different port to avoid conflicts
        let server_url = format!("http://{}:{}", address, port);

        // Start server in background
        let server_handle = tokio::spawn(async move {
            let messages = Arc::new(Mutex::new(Vec::new()));
            let clients = Arc::new(Mutex::new(Vec::new()));
            let users = Arc::new(Mutex::new(HashMap::new()));
            let app_state = AppState {
                messages,
                clients,
                users,
            };

            let socket_addr: SocketAddr = format!("{}:{}", address, port).parse().unwrap();
            let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();

            let app = Router::new()
                .route("/room/1", get(handle_websocket))
                .route("/room/1", post(handle_post))
                .route("/messages", get(handle_get))
                .with_state(app_state);

            axum::serve(listener, app).await.unwrap();
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Test that server is responding
        let client = reqwest::Client::new();
        let response = client.get(&format!("{}/messages", server_url)).send().await;

        assert!(response.is_ok());
        assert!(response.unwrap().status().is_success());

        // Stop server by aborting the task
        server_handle.abort();

        // Give server time to stop
        sleep(Duration::from_millis(100)).await;

        // Test that server is no longer responding
        let response = client.get(&format!("{}/messages", server_url)).send().await;

        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_server_with_multiple_clients() {
        let address = "127.0.0.1";
        let port = 12347; // Use different port
        let server_url = format!("http://{}:{}", address, port);

        // Start server in background
        let server_handle = tokio::spawn(async move {
            let messages = Arc::new(Mutex::new(Vec::new()));
            let clients = Arc::new(Mutex::new(Vec::new()));
            let users = Arc::new(Mutex::new(HashMap::new()));
            let app_state = AppState {
                messages,
                clients,
                users,
            };

            let socket_addr: SocketAddr = format!("{}:{}", address, port).parse().unwrap();
            let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();

            let app = Router::new()
                .route("/room/1", get(handle_websocket))
                .route("/room/1", post(handle_post))
                .route("/messages", get(handle_get))
                .with_state(app_state);

            axum::serve(listener, app).await.unwrap();
        });

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();

        // Test multiple client connections via HTTP endpoints
        for i in 0..3 {
            let message = Message {
                text: format!("Client {} message", i),
            };

            let response = client
                .post(&format!("{}/room/1", server_url))
                .json(&message)
                .send()
                .await;

            assert!(response.is_ok());
            assert!(response.unwrap().status().is_success());
        }

        // Verify messages were stored
        let response = client
            .get(&format!("{}/messages", server_url))
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());
        let content = response.text().await.unwrap();
        assert!(content.contains("Client 0 message"));
        assert!(content.contains("Client 1 message"));
        assert!(content.contains("Client 2 message"));

        // Stop server
        server_handle.abort();
    }
}

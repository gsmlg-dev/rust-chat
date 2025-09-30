use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use futures::{sink::SinkExt, stream::StreamExt};
use std::collections::HashMap;
use std::time::{Duration, Instant};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub connected_at: Instant,
}

#[derive(Clone)]
pub struct AppState {
    pub messages: Arc<Mutex<Vec<Message>>>,
    pub clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<Message>>>>,
    pub users: Arc<Mutex<HashMap<String, User>>>,
}

pub async fn run_server(address: &str, port: u16, tui: bool) {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let clients = Arc::new(Mutex::new(Vec::new()));
    let users = Arc::new(Mutex::new(HashMap::new()));
    let app_state = AppState { messages, clients, users };
    
    let addr = format!("{}:{}", address, port);
    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");
    
    if tui {
        println!("Chat server running on http://{} with TUI", socket_addr);
        run_tui_server(app_state.clone(), socket_addr).await;
    } else {
        println!("Chat server running on http://{}", socket_addr);
let app = Router::new()
            .route("/room/1", get(handle_websocket))
            .route("/room/1", post(handle_post))
            .route("/messages", get(handle_get))
            .with_state(app_state);
        let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Generate a unique user ID
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User {
        id: user_id.clone(),
        name: format!("User_{}", user_id.split('-').next().unwrap()),
        connected_at: Instant::now(),
    };
    
    // Add this client to list
    state.clients.lock().unwrap().push(tx);
    
    // Add user to tracking
    {
        let mut users = state.users.lock().unwrap();
        users.insert(user_id.clone(), user.clone());
    }
    
    // Send existing messages to new client
    let messages_to_send = {
        let messages = state.messages.lock().unwrap();
        messages.iter().map(|msg| msg.text.clone()).collect::<Vec<String>>()
    };
    
    for msg_text in messages_to_send {
        if sender.send(axum::extract::ws::Message::Text(msg_text.into())).await.is_err() {
            return;
        }
    }
    
    // Handle incoming messages from this client
    let state_clone = state.clone();
    let recv_task = async {
        while let Some(msg) = receiver.next().await {
            if let Ok(axum::extract::ws::Message::Text(text)) = msg {
                let message = Message { text: text.to_string() };
                
                // Store message
                {
                    let mut messages = state_clone.messages.lock().unwrap();
                    messages.push(message.clone());
                }
                
                // Broadcast to all clients
                let clients = state_clone.clients.lock().unwrap();
                for client_tx in clients.iter() {
                    let _ = client_tx.send(message.clone());
                }
            }
        }
    };
    
    // Handle outgoing messages to this client
    let send_task = async {
        while let Some(msg) = rx.recv().await {
            if sender.send(axum::extract::ws::Message::Text(msg.text.into())).await.is_err() {
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
}

async fn handle_get(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let messages = state.messages.lock().unwrap();
    let response: String = messages.iter()
        .map(|msg| format!("{}\n", msg.text))
        .collect();
    
    (StatusCode::OK, response)
}

async fn handle_post(
    State(state): State<AppState>,
    Json(message): Json<Message>,
) -> impl IntoResponse {
    let mut messages = state.messages.lock().unwrap();
    messages.push(message.clone());
    
    // Broadcast to all WebSocket clients
    let clients = state.clients.lock().unwrap();
    for client_tx in clients.iter() {
        let _ = client_tx.send(message.clone());
    }
    
    StatusCode::CREATED
}

async fn run_tui_server(state: AppState, socket_addr: SocketAddr) {
    let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
    let state_clone = state.clone();
    
    // Start the server in a separate task
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, {
            let app = Router::new()
                .route("/room/1", get(handle_websocket))
                .route("/room/1", post(handle_post))
                .route("/messages", get(handle_get))
                .with_state(state_clone.clone());
            app
        }).await.unwrap();
    });
    
    // Run TUI
    if let Err(e) = run_tui(state).await {
        eprintln!("TUI error: {}", e);
    }
    
    // Cancel server task
    server_handle.abort();
}

async fn run_tui(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    println!("TUI Mode - Press Ctrl+C to exit");
    
    loop {
        // Clear screen and print status
        print!("\x1B[2J\x1B[H"); // Clear screen and move cursor to top
        
        let users = state.users.lock().unwrap();
        
        println!("┌─────────────────────────────────────────┐");
        println!("│ Chat Server TUI                          │");
        println!("├─────────────────────────────────────────┤");
        println!("│ Connected Users: {}                     │", users.len());
        println!("├─────────────────────────────────────────┤");
        
        if users.is_empty() {
            println!("│ No users connected                      │");
        } else {
            for (i, user) in users.values().enumerate() {
                let duration = user.connected_at.elapsed();
                if i < 10 { // Limit display to 10 users
                    println!("│ {} ({}s ago)                       │", 
                        user.name.chars().take(30).collect::<String>(), 
                        duration.as_secs());
                }
            }
            if users.len() > 10 {
                println!("│ ... and {} more users                  │", users.len() - 10);
            }
        }
        
        println!("├─────────────────────────────────────────┤");
        println!("│ Press Ctrl+C to quit                    │");
        println!("└─────────────────────────────────────────┘");
        
        // Sleep for 1 second before refreshing
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
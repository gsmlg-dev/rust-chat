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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
}

#[derive(Clone)]
pub struct AppState {
    pub messages: Arc<Mutex<Vec<Message>>>,
    pub clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

pub async fn run_server(address: &str, port: u16) {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let clients = Arc::new(Mutex::new(Vec::new()));
    let app_state = AppState { messages, clients };
    
    let app = Router::new()
        .route("/room/1", get(handle_websocket))
        .route("/room/1", post(handle_post))
        .route("/room/1", get(handle_get))
        .with_state(app_state);
    
    let addr = format!("{}:{}", address, port);
    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");
    
    println!("Chat server running on http://{}", socket_addr);
    
    let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    
    // Add this client to list
    state.clients.lock().unwrap().push(tx);
    
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
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

/// Custom error types for the chat application
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ChatError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
}

pub type ChatResult<T> = Result<T, ChatError>;

/// Represents a chat message sent between clients and server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The text content of the message
    pub text: String,
}

/// Represents a list of users currently connected to the chat
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents the list of users currently connected to the chat.
///
/// This structure is used to send user list updates to clients,
/// containing both the user count and the list of user information.
pub struct UserList {
    /// List of connected users (serializable format for network transmission)
    pub users: Vec<SerializableUser>,
    /// Total number of connected users
    pub count: usize,
}

/// A serializable version of User for transmission over the network.
///
/// This struct contains only the user information that needs to be
/// sent to clients, excluding sensitive data like the full UUID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableUser {
    /// The user's display name
    pub name: String,
}

/// Represents a user connected to the chat server
/// Represents a user in the chat system.
///
/// Each user has a unique ID, display name, and connection timestamp.
/// The user ID is automatically generated when the user is created.
#[derive(Debug, Clone)]
pub struct User {
    /// Unique identifier for the user (UUID v4)
    #[allow(dead_code)]
    pub id: String,
    /// The user's display name in the chat
    pub name: String,
    /// Timestamp when the user connected to the server
    pub connected_at: Instant,
}

/// Message types for client-server communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Regular chat message
    Chat { text: String },
    /// User list update
    UserList(UserList),
    /// User joined notification
    UserJoined { name: String },
    /// User left notification
    UserLeft { name: String },
}

/// Message types for client-to-server communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Initial connection message with user name
    Connect { name: String },
    /// Regular chat message
    Chat { text: String },
    /// Disconnect notification
    Disconnect,
}

impl From<&User> for SerializableUser {
    fn from(user: &User) -> Self {
        SerializableUser {
            name: user.name.clone(),
        }
    }
}

impl Message {
    /// Create a new message with the given text
    #[allow(dead_code)]
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// Create a formatted chat message with sender name
    pub fn chat_message(sender: &str, text: &str) -> Self {
        Self {
            text: format!("{}: {}", sender, text),
        }
    }
}

impl User {
    /// Create a new user with the given name and generated ID
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            connected_at: Instant::now(),
        }
    }
}

impl UserList {
    /// Create a new user list from a collection of users
    pub fn from_users(users: &[User]) -> Self {
        let serializable_users: Vec<SerializableUser> = users.iter().map(|u| u.into()).collect();
        Self {
            users: serializable_users,
            count: users.len(),
        }
    }
}

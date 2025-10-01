use futures::{sink::SinkExt, stream::StreamExt};
use rustyline::Editor;
use rustyline::error::ReadlineError;
use std::io::Write;

use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

use crate::shared::{Message, UserList, ClientMessage, ServerMessage, ChatError, ChatResult};

/// Runs the chat client and connects to the specified server.
/// 
/// This function establishes a WebSocket connection to the chat server,
/// handles user input, and displays incoming messages in real-time.
/// 
/// # Arguments
/// 
/// * `server_address` - The IP address or hostname of the chat server
/// * `server_port` - The port number the server is listening on
/// * `name` - Optional username for the client. If None, a random name is generated.
/// 
/// # Examples
/// 
/// ```rust
/// // Connect with a specific name
/// run_client("127.0.0.1", 12345, Some("Alice".to_string())).await;
/// 
/// // Connect with a random name
/// run_client("127.0.0.1", 12345, None).await;
/// ```
pub async fn run_client(server_address: &str, server_port: u16, name: Option<String>) {
    let client_name = name.unwrap_or_else(generate_random_name);
    let ws_url = format!("ws://{}:{}/room/1", server_address, server_port);

    println!("Connecting to chat server as {}...", client_name);

    let (ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect to server");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Send initial connection message with user name
    let connect_msg = ClientMessage::Connect { name: client_name.clone() };
    let json = serde_json::to_string(&connect_msg).expect("Failed to serialize connect message");
    ws_sender
        .send(WsMessage::Text(json.into()))
        .await
        .expect("Failed to send connect message");

    let _tx_clone = tx.clone();
    let client_name_clone = client_name.clone();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let chat_msg = ClientMessage::Chat { text: msg };
            let json = serde_json::to_string(&chat_msg).expect("Failed to serialize chat message");
            ws_sender
                .send(WsMessage::Text(json.into()))
                .await
                .expect("Failed to send message");
        }
    });

    let _tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    // Try to parse as ServerMessage
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        match server_msg {
                            ServerMessage::Chat { text } => {
                                let mut t = term::stdout().unwrap();
                                t.fg(term::color::GREEN).unwrap();
                                writeln!(t, "{}", text).unwrap();
                                t.reset().unwrap();
                            }
                            ServerMessage::UserList(user_list) => {
                                let mut t = term::stdout().unwrap();
                                t.fg(term::color::BLUE).unwrap();
                                writeln!(t, "=== Users online: {} ===", user_list.count).unwrap();
                                for user in user_list.users {
                                    writeln!(t, "  {}", user.name).unwrap();
                                }
                                writeln!(t, "========================").unwrap();
                                t.reset().unwrap();
                            }
                            ServerMessage::UserJoined { name } => {
                                let mut t = term::stdout().unwrap();
                                t.fg(term::color::YELLOW).unwrap();
                                writeln!(t, "*** {} joined the chat ***", name).unwrap();
                                t.reset().unwrap();
                            }
                            ServerMessage::UserLeft { name } => {
                                let mut t = term::stdout().unwrap();
                                t.fg(term::color::YELLOW).unwrap();
                                writeln!(t, "*** {} left the chat ***", name).unwrap();
                                t.reset().unwrap();
                            }
                        }
                    } else {
                        // Fallback for old message format
                        let mut t = term::stdout().unwrap();
                        t.fg(term::color::GREEN).unwrap();
                        writeln!(t, "{}", text).unwrap();
                        t.reset().unwrap();
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("Server closed connection");
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    run_chat_tui(tx, &client_name).await;
}

fn generate_random_name() -> String {
    let adjectives = [
        "Happy", "Quick", "Silent", "Brave", "Clever", "Swift", "Bright", "Calm",
    ];
    let nouns = [
        "Panda", "Eagle", "Tiger", "Wolf", "Fox", "Bear", "Lion", "Hawk",
    ];

    let adj = adjectives[rand::random::<usize>() % adjectives.len()];
    let noun = nouns[rand::random::<usize>() % nouns.len()];
    let number = rand::random::<u32>() % 1000;

    format!("{}{}{}", adj, noun, number)
}

async fn run_chat_tui(tx: mpsc::UnboundedSender<String>, client_name: &str) {
    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new().unwrap();

    println!(
        "Chat started as {}. Type your messages and press Enter.",
        client_name
    );
    println!("Press Ctrl+C to exit.");

    loop {
        let readline = rl.readline(&format!("{}: ", client_name));
        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                if tx.send(line).is_err() {
                    eprintln!("Failed to send message");
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Exiting chat...");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("Exiting chat...");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use url::Url;

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
    async fn test_empty_message_handling() {
        let message = Message {
            text: "".to_string(),
        };

        assert_eq!(message.text, "");

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, "");
    }

    #[tokio::test]
    async fn test_long_message_handling() {
        let long_text = "a".repeat(1000);
        let message = Message {
            text: long_text.clone(),
        };

        assert_eq!(message.text.len(), 1000);

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text.len(), 1000);
        assert_eq!(deserialized.text, long_text);
    }

    #[tokio::test]
    async fn test_special_characters_in_message() {
        let special_text = "Hello! @#$%^&*()_+{}|:\"<>?[]\\;',./`~";
        let message = Message {
            text: special_text.to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, special_text);
    }

    #[tokio::test]
    async fn test_unicode_characters_in_message() {
        let unicode_text = "Hello 世界! 🚀";
        let message = Message {
            text: unicode_text.to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, unicode_text);
    }

    #[tokio::test]
    async fn test_name_formatting_edge_cases() {
        // Test empty name
        let name = "";
        let line = "Hello";
        let mut whom = String::from(name);
        whom.push_str(": ");
        whom.push_str(line);
        assert_eq!(whom, ": Hello");

        // Test name with spaces
        let name = "John Doe";
        let line = "Hello";
        let mut whom = String::from(name);
        whom.push_str(": ");
        whom.push_str(line);
        assert_eq!(whom, "John Doe: Hello");

        // Test name with special characters
        let name = "User@123";
        let line = "Hello";
        let mut whom = String::from(name);
        whom.push_str(": ");
        whom.push_str(line);
        assert_eq!(whom, "User@123: Hello");
    }

    #[tokio::test]
    async fn test_random_name_generation() {
        let name1 = generate_random_name();
        let name2 = generate_random_name();

        // Names should be strings
        assert!(!name1.is_empty());
        assert!(!name2.is_empty());

        // Names should likely be different (very high probability)
        // We don't require them to be different since it's random, but they usually will be
        println!("Generated names: {}, {}", name1, name2);

        // Name should contain alphanumeric characters
        assert!(name1.chars().all(|c| c.is_alphanumeric()));
        assert!(name2.chars().all(|c| c.is_alphanumeric()));
    }

    #[tokio::test]
    async fn test_websocket_url_construction() {
        let address = "127.0.0.1";
        let port = 8080;
        let ws_url = format!("ws://{}:{}/room/1", address, port);

        assert_eq!(ws_url, "ws://127.0.0.1:8080/room/1");

        // Test URL parsing
        let url = Url::parse(&ws_url);
        assert!(url.is_ok());

        let parsed_url = url.unwrap();
        assert_eq!(parsed_url.scheme(), "ws");
        assert_eq!(parsed_url.host_str().unwrap(), "127.0.0.1");
        assert_eq!(parsed_url.port().unwrap(), 8080);
        assert_eq!(parsed_url.path(), "/room/1");
    }

    #[tokio::test]
    async fn test_client_message_formatting() {
        let client_name = "Alice";
        let message_text = "Hello everyone!";

        let formatted_message = format!("{}: {}", client_name, message_text);
        assert_eq!(formatted_message, "Alice: Hello everyone!");

        let message = Message {
            text: formatted_message.clone(),
        };

        assert_eq!(message.text, "Alice: Hello everyone!");
    }
}

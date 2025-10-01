use std::thread;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::io::Write;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
}

pub async fn run_client(name: &str, server_address: &str, server_port: u16) {
    let server_url = format!("http://{}:{}", server_address, server_port);
    let server_url_clone = server_url.clone();
    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new().unwrap();
    
    thread::spawn(move || {
        loop {
            match reqwest::blocking::get(&format!("{}/messages", server_url_clone)) {
                Ok(mut resp) => {
                    let mut t = term::stdout().unwrap();
                    
                    if resp.status().is_success() {
                        let mut content = String::new();
                        use std::io::Read;
                        if resp.read_to_string(&mut content).is_ok() {
                            t.fg(term::color::GREEN).unwrap();
                            write!(t, "{}", content).unwrap();
                            t.reset().unwrap();
                        }
                    } else if resp.status().is_server_error() {
                        t.fg(term::color::RED).unwrap();
                        writeln!(t, "Server error! Status: {:?}", resp.status()).unwrap();
                        t.reset().unwrap();
                    } else {
                        t.fg(term::color::CYAN).unwrap();
                        writeln!(t, "Something else happened. Status: {:?}", resp.status()).unwrap();
                        t.reset().unwrap();
                    }
                }
                Err(e) => {
                    let mut t = term::stdout().unwrap();
                    t.fg(term::color::RED).unwrap();
                    writeln!(t, "Connection error: {:?}", e).unwrap();
                    t.reset().unwrap();
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
    
    println!("Start Chat as {:?} connecting to {}", name, server_url);
    loop {
        let mut who = String::from(name);
        who.push_str(": ");
        let readline = rl.readline(who.as_str());
        match readline {
            Ok(line) => {
                let client = reqwest::Client::new();
                let mut whom = String::from(name);
                whom.push_str(": ");
                whom.push_str(&line.clone());
                
                let message = Message { text: whom.clone() };
                
                match client.post(&format!("{}/room/1", server_url))
                    .json(&message)
                    .send()
                    .await {
                    Ok(_) => (),
                    Err(e) => {
                        let mut t = term::stdout().unwrap();
                        t.fg(term::color::RED).unwrap();
                        writeln!(t, "Failed to send message: {:?}", e).unwrap();
                        t.reset().unwrap();
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};

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
    async fn test_client_connect_to_server() {
        let address = "127.0.0.1";
        let port = 12348; // Use different port
        let server_url = format!("http://{}:{}", address, port);
        
        // Start a mock server
        let server_handle = tokio::spawn(async move {
            let app = axum::Router::new()
                .route("/messages", get(|| async { "No messages yet\n" }))
                .route("/room/1", post(|| async { axum::http::StatusCode::CREATED }));
                
            let listener = tokio::net::TcpListener::bind(format!("{}:{}", address, port))
                .await
                .unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        
        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Test client connection by making HTTP requests
        let client = reqwest::Client::new();
        
        // Test GET messages endpoint
        let response = client.get(&format!("{}/messages", server_url))
            .send()
            .await;
            
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.status().is_success());
        
        // Test POST message endpoint
        let message = Message {
            text: "TestClient: Hello".to_string(),
        };
        
        let response = client.post(&format!("{}/room/1", server_url))
            .json(&message)
            .send()
            .await;
            
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        
        // Stop server
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_client_disconnect_handling() {
        let address = "127.0.0.1";
        let port = 12349; // Use different port
        let server_url = format!("http://{}:{}", address, port);
        
        // Start a mock server that tracks connections
        let server_handle = tokio::spawn(async move {
            let app = axum::Router::new()
                .route("/messages", get(|| async { "Test messages\n" }))
                .route("/room/1", post(|| async { axum::http::StatusCode::CREATED }));
                
            let listener = tokio::net::TcpListener::bind(format!("{}:{}", address, port))
                .await
                .unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        
        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let client = reqwest::Client::new();
        
        // Simulate client connecting and making requests
        for _i in 0..3 {
            let response = client.get(&format!("{}/messages", server_url))
                .send()
                .await;
                
            assert!(response.is_ok());
            assert!(response.unwrap().status().is_success());
            
            // Small delay between requests
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        // Simulate disconnection by dropping the client
        drop(client);
        
        // Give time for connection to be fully closed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Create new client to verify server is still running
        let new_client = reqwest::Client::new();
        let response = new_client.get(&format!("{}/messages", server_url))
            .send()
            .await;
            
        assert!(response.is_ok());
        assert!(response.unwrap().status().is_success());
        
        // Stop server
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_client_connection_error_handling() {
        let address = "127.0.0.1";
        let port = 12350; // Use port that won't have a server running
        let server_url = format!("http://{}:{}", address, port);
        
        let client = reqwest::Client::new();
        
        // Test connection to non-existent server
        let response = client.get(&format!("{}/messages", server_url))
            .send()
            .await;
            
        assert!(response.is_err());
        
        // Test POST to non-existent server
        let message = Message {
            text: "TestClient: Hello".to_string(),
        };
        
        let response = client.post(&format!("{}/room/1", server_url))
            .json(&message)
            .send()
            .await;
            
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_client_multiple_connections() {
        let address = "127.0.0.1";
        let port = 12351; // Use different port
        let server_url = format!("http://{}:{}", address, port);
        
        // Start a mock server
        let server_handle = tokio::spawn(async move {
            let app = axum::Router::new()
                .route("/messages", get(|| async { "Server messages\n" }))
                .route("/room/1", post(|| async { axum::http::StatusCode::CREATED }));
                
            let listener = tokio::net::TcpListener::bind(format!("{}:{}", address, port))
                .await
                .unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        
        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Create multiple clients
        let client1 = reqwest::Client::new();
        let client2 = reqwest::Client::new();
        let client3 = reqwest::Client::new();
        
        // All clients should be able to connect
        let response1 = client1.get(&format!("{}/messages", server_url)).send().await;
        let response2 = client2.get(&format!("{}/messages", server_url)).send().await;
        let response3 = client3.get(&format!("{}/messages", server_url)).send().await;
        
        assert!(response1.is_ok());
        assert!(response2.is_ok());
        assert!(response3.is_ok());
        
        assert!(response1.unwrap().status().is_success());
        assert!(response2.unwrap().status().is_success());
        assert!(response3.unwrap().status().is_success());
        
        // All clients should be able to post messages
        let message1 = Message { text: "Client1: Hello".to_string() };
        let message2 = Message { text: "Client2: Hello".to_string() };
        let message3 = Message { text: "Client3: Hello".to_string() };
        
        let post1 = client1.post(&format!("{}/room/1", server_url)).json(&message1).send().await;
        let post2 = client2.post(&format!("{}/room/1", server_url)).json(&message2).send().await;
        let post3 = client3.post(&format!("{}/room/1", server_url)).json(&message3).send().await;
        
        assert!(post1.is_ok());
        assert!(post2.is_ok());
        assert!(post3.is_ok());
        
        assert_eq!(post1.unwrap().status(), reqwest::StatusCode::CREATED);
        assert_eq!(post2.unwrap().status(), reqwest::StatusCode::CREATED);
        assert_eq!(post3.unwrap().status(), reqwest::StatusCode::CREATED);
        
        // Stop server
        server_handle.abort();
    }
}
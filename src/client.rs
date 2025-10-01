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
}
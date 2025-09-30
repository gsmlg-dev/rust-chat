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
            match reqwest::blocking::get(&format!("{}/room/1", server_url_clone)) {
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
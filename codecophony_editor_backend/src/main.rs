extern crate codecophony;
extern crate codecophony_editor_shared as shared;
extern crate serde_json;

use std::thread;
use std::io::{self, BufRead, Write};
//use std::time::Duration;

use shared::{MessageToBackend, MessageToFrontend};


fn main() {
  //println!("Hello from backend (stdout)");
  eprintln!("Hello from backend (stderr)");
  let stdin = io::stdin();
  let stdin = stdin.lock();
  let stdout = io::stdout();
  let mut stdout = stdout.lock();
  for line in stdin.lines() {
    let line = line.unwrap();
    if let Ok(message) = serde_json::from_str(&line) {
      match message {
        MessageToBackend::Echo (text) => {
          serde_json::to_writer (&mut stdout, & MessageToFrontend::Print(text)).unwrap();
          write!(stdout, "\n").unwrap();
        },
      }
    }
    else {
      eprintln!("Received invalid message from frontend: {}", line);
    }
    
    //thread::sleep(Duration::from_millis(50));
  }
}

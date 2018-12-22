extern crate codecophony;
extern crate codecophony_editor_shared as shared;
extern crate serde_json;
extern crate portaudio;
extern crate dsp;

//use std::thread;
use std::io::{self, BufRead, Write};
//use std::time::Duration;

use dsp::sample::ToFrameSliceMut;
use portaudio::{PortAudio, Stream, StreamSettings};

use codecophony::Renderable;
use shared::{MessageToBackend, MessageToFrontend};

type Output = f32;
const CHANNELS: usize = 2;
const SAMPLE_HZ: f64 = 44100.0;

fn main() {
  //println!("Hello from backend (stdout)");
  eprintln!("Hello from backend (stderr)");
  let stdin = io::stdin();
  let stdin = stdin.lock();
  let stdout = io::stdout();
  let mut stdout = stdout.lock();
  
  let callback = move |portaudio::OutputStreamCallbackArgs { buffer, .. }| {
    let buffer: &mut [[Output; CHANNELS]] = buffer.to_frame_slice_mut().unwrap();
    dsp::slice::equilibrium(buffer);
      
    let position = 0;
    let note = codecophony::MIDIPercussionNote::new (0.0, 10.0, 100, 36);
    Renderable::<[Output; CHANNELS]>::render(&note, buffer, position, SAMPLE_HZ);
    
    portaudio::Continue
  };
  
  let pa = portaudio::PortAudio::new().unwrap();
  let settings = pa.default_output_stream_settings::<Output>(
    CHANNELS as i32,
    SAMPLE_HZ,
    4096, // frames per buffer
  ).unwrap();
  let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();
  stream.start().unwrap();

  
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


extern crate codecophony;
extern crate codecophony_editor_shared as shared;
extern crate serde_json;
extern crate portaudio;
extern crate dsp;

//use std::thread;
use std::io::{self, BufRead, Write};
//use std::time::Duration;

use dsp::sample::ToFrameSliceMut;
use dsp::Frame as DspFrame;
use portaudio::{PortAudio, Stream, StreamSettings, PaStreamCallbackResult};

use codecophony::{Renderable];
use shared::{MessageToBackend, MessageToFrontend};

type Output = f32;
const CHANNELS: usize = 2;
const SAMPLE_HZ: f64 = 44100.0;
const FRAMES_PER_BUFFER: usize = 4096;
const FADE_FRAMES = 100;
type Frame = [Output; CHANNELS];

struct Rend{
  
}

enum MessageToRenderThread {

}

struct PortaudioThread {
  
}

impl PortaudioThread {
  fn fill_buffer_no_wrap_or_fade(&mut self, output_buffer: &mut [Frame], start: usize, virtual_length: usize)->usize {
    let max_duration = min(virtual_length, self.playback.end - start);
    if max_duration == 0 {
      return start;
    }
    let next_chunk_index = start / FRAMES_PER_BUFFER;
    if let Some(rendered_chunk) = self.rendered_chunks.get (next_chunk_index) {
      let chunk_start = next_chunk_index * FRAMES_PER_BUFFER;
      let start_within_chunk = start-chunk_start;
      let virtual_duration_copied = min(max_duration, rendered_chunk.len() - start_within_chunk);
      let true_duration_copied = min(virtual_duration_copied, output_buffer.len());
      output_buffer[..true_duration_copied].copy_from_slice (&rendered_chunk [start_within_chunk..start_within_chunk+true_duration_copied]);
      fill_buffer(&mut output_buffer[true_duration_copied..], start + virtual_duration_copied, virtual_length - virtual_duration_copied)
    }
    else {
      start
    }
  }
  
  fn fill_buffer(&mut self, output_buffer: &mut [Frame]) {
    let old_start = self.playback.next_start;
    let virtual_length = output_buffer.len() + FADE_FRAMES;
    let virtual_stopped_before = self.fill_buffer_no_wrap_or_fade(output_buffer, self.playback.next_start, virtual_length);
    let virtual_frames_copied = stopped_before - old_start;
    let finished = virtual_frames_copied == virtual_length;
    self.playback.next_start = min (stopped_before, old_start + output_buffer.len());
    
    if finished {
      self.next_amplitude_adjustment = 1.0;
    }
    else {
      for fade_index in 0..FADE_FRAMES {
        let frame_index = virtual_frames_copied - 1 - fade_index;
        let amplitude = fade_index as f64 / FADE_FRAMES as f64;
        if frame_index == output_buffer.len() {
          self.next_amplitude_adjustment = 0.0;
        }
      }
      self.next_amplitude_adjustment = 0.0;
    
      if self.playback.next_start == self.playback.end && self.playback.loop {
        self.playback.next_start = self.playback.loop_from;
        
        fill_buffer (output_buffer [stopped_before..])
      }
    }
  }

  fn call(&mut self, portaudio::OutputStreamCallbackArgs { buffer, .. }) -> PaStreamCallbackResult {
    let output_buffer: &mut [Frame] = buffer.to_frame_slice_mut().unwrap();
    
    
    let found_buffers: ArrayVec<[&[Frame]]> = ArrayVec::new();
    let (start, end) = (self.playback.next_start, self.playback.next_start + FRAMES_PER_BUFFER);
    let mut found_until = self.playback.next_start;
    for chunk_index in self.playback.next_start / FRAMES_PER_BUFFER .. (self.playback.next_start + FRAMES_PER_BUFFER - 1) / FRAMES_PER_BUFFER {
      if let Some(rendered_buffer) = self.rendered_buffers.get (chunk_index) {
        found_buffers.push(rendered_buffer);
        found_until = (chunk_index+1)*FRAMES_PER_BUFFER;
      }
      else {
        break;
      }
    }
    
    let mut fadeout = found_until < self.playback.next_start + FRAMES_PER_BUFFER;
    dsp::slice::equilibrium(output_buffer[]);
    
    let next_playback = self.playback.clone();
    if let Ok(message) = self.render_recv.try_recv() {
      match message {
        MessageToRenderThread::ChangePlayback(playback) {
          next_playback = playback;
          fadeout = true;
        }
      }
    }
    

    If changing {
    
    }
    portaudio::Continue
  }
}

fn main() {
  //println!("Hello from backend (stdout)");
  eprintln!("Hello from backend (stderr)");
  let stdin = io::stdin();
  let stdin = stdin.lock();
  let stdout = io::stdout();
  let mut stdout = stdout.lock();
  
  let callback = move |portaudio::OutputStreamCallbackArgs { buffer, .. }| {
    let buffer: &mut [Frame] = buffer.to_frame_slice_mut().unwrap();
   
    dsp::slice::equilibrium(buffer);
      
    let position = 0;
    let note = codecophony::MIDIPercussionNote::new (0.0, 10.0, 100, 36);
    Renderable::<Frame>::render(&note, buffer, position, SAMPLE_HZ);
    
    portaudio::Continue
  };
  
  let pa = portaudio::PortAudio::new().unwrap();
  let settings = pa.default_output_stream_settings::<Output>(
    CHANNELS as i32,
    SAMPLE_HZ,
    FRAMES_PER_BUFFER,
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


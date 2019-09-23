use std::thread;
use std::io::{self, BufRead, Write};
use std::sync::mpsc::{Sender, Receiver, channel};
//use std::time::Duration;
use std::cmp::{min, max};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use dsp::sample::{ToFrameSliceMut, Frame as DspFrame};
use portaudio::{StreamCallbackResult, Stream, NonBlocking};
use ordered_float::NotNan;

use codecophony::{FrameTime, FluidsynthDirectlyRenderableMIDINote, FluidsynthDirectlyRenderableMIDIInstrument};

type Output = f32;
const CHANNELS: usize = 2;
const SAMPLE_HZ: f64 = 44100.0;
const FRAMES_PER_BUFFER: usize = 256;
const FADEIN_TIME: f64 = 0.01;
const FADEIN_FRAMES: FrameTime = (FADEIN_TIME*SAMPLE_HZ) as FrameTime;
const FADEOUT_TIME: f64 = 0.25;
const FADEOUT_FRAMES: FrameTime = (FADEOUT_TIME*SAMPLE_HZ) as FrameTime;
type Frame = [Output; CHANNELS];

fn note_frames(note: & Note)->Arc <[Frame]> {
  codecophony::with_rendered_midi_note (& FluidsynthDirectlyRenderableMIDINote {
    duration: NotNan::new(note.duration).unwrap(),
    pitch: note.pitch,
    velocity: 100,
    instrument: FluidsynthDirectlyRenderableMIDIInstrument::pitched (1),
  }, SAMPLE_HZ, | frames| frames.clone())
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Note {
  pub start_time: f64,
  pub duration: f64,
  pub pitch: i32,
}

impl Note {
  pub fn end_time(&self)->f64 {self.start_time + self.duration}
}


#[derive(Serialize, Deserialize, Default)]
pub struct PlaybackScript {
  pub notes: Vec<Note>,
  pub end: Option<f64>,
  pub loop_back_to: Option<f64>,
}

struct PortaudioThread {
  notes: Vec<PlaybackNote>,
  next_frame_time: FrameTime,
  receiver: Receiver <MessageToPortaudioThread>,
  sender: Sender <MessageToRenderThread>,
}


enum MessageToPortaudioThread {
  Stop,
  AddNote(PlaybackNote),
}
pub enum MessageToRenderThread {
  PlaybackReachedTime(FrameTime),
  ReplaceScript (PlaybackScript),
  RestartPlaybackAt (Option<f64>),
}

struct PlaybackNote {
  frames: Arc<[Frame]>,
  first_sample_time: FrameTime,
  
  fadein_start: Option<FrameTime>,
  fadeout_end: Option<FrameTime>,
}

impl PlaybackNote {
  fn start_time(&self)->FrameTime {self.fadein_start.unwrap_or (self.first_sample_time)}
  fn end_time(&self)->FrameTime {self.fadeout_end.unwrap_or (self.first_sample_time + self.frames.len() as FrameTime)}
}

impl PortaudioThread {
  fn call(&mut self, portaudio::OutputStreamCallbackArgs { buffer: output_buffer, .. }: portaudio::OutputStreamCallbackArgs <Output>) -> StreamCallbackResult {
    let output_buffer: &mut [Frame] = output_buffer.to_frame_slice_mut().unwrap();
    dsp::slice::equilibrium(output_buffer);
    
    while let Ok (message) = self.receiver.try_recv() {
      match message {
        MessageToPortaudioThread::Stop => {
          let time = self.next_frame_time;
          let finish = time + FADEOUT_FRAMES;
          self.notes.retain(|note| note.start_time() < time);
          for note in &mut self.notes {
            if finish < note.end_time() {
              note.fadeout_end = Some (note.fadeout_end.map_or(finish, |a| min(a, finish)));
            }
          }
        },
        MessageToPortaudioThread::AddNote (note) => {
          self.notes.push (note);
        },
      }
    }
    
    let end_time = self.next_frame_time + FRAMES_PER_BUFFER as FrameTime;
    
    for note in &self.notes {
      
      let note_start_time = max (note.start_time(), self.next_frame_time);
      let note_end_time = min (note.end_time(), end_time);
      let start_sample_offset = note_start_time - note.first_sample_time;
      let duration = note_end_time - note_start_time;
      let end_sample_offset = start_sample_offset + duration;
      //eprintln!(" doing note {:?} ", (duration, end_time, note.end_time()));
      if duration <= 0 {continue;}
      for (frame_time, (output, sample)) in
        (note_start_time..note_end_time)
        .zip (
          output_buffer [(note_start_time - self.next_frame_time) as usize..(note_end_time - self.next_frame_time) as usize].iter_mut()
          .zip (
            & note.frames[start_sample_offset as usize..end_sample_offset as usize]
          )
        ) {
        let mut factor = 1.0;
        if let Some(fadein_start) = note.fadein_start {
          factor *= max(0, min(FADEIN_FRAMES, frame_time - fadein_start)) as f32/ FADEIN_FRAMES as f32;
        }
        if let Some(fadeout_end) = note.fadeout_end {
          factor *= max(0, min(FADEOUT_FRAMES, fadeout_end - frame_time)) as f32/ FADEOUT_FRAMES as f32;
        }
        *output = output.add_amp(sample.scale_amp(factor));
      }
    }
    
    
    
    self.notes.retain(|note| {
      note.end_time() > end_time
    });
    
    self.next_frame_time = end_time;
    //eprintln!("output {:?}", output_buffer);
    self.sender.send (MessageToRenderThread::PlaybackReachedTime(end_time)).unwrap() ;
    
    portaudio::Continue
  }
}

/*enum PlaybackProgressState {
  StalledAtScriptTime(f64),
  Playing{script_start: f64, frame_start: FrameTime},
}*/

struct RenderPreparingNote {
  note: Note,
  duration_from_start: f64,
}


struct RenderThread {
  receiver: Receiver <MessageToRenderThread>,
  portaudio_sender: Sender <MessageToPortaudioThread>,
  //main_sender: Sender <MessageToFrontend>,
  script: PlaybackScript,
  render_queue: Vec<RenderPreparingNote>,
  playback_queue: Vec<RenderPreparingNote>,
  latest_playback_reached: FrameTime,
  renders_queued_until: f64, // duration from stall/start
  specified_playback_start_script_time: Option<f64>,
  actual_playback_start_frame_time: Option<FrameTime>,
}

impl RenderThread {
  fn render_loop (&mut self) {
    while let Ok (message) = self.receiver.recv() {
      self.process_message (message);
      while self.render_step() {
        if let Ok (message) = self.receiver.try_recv() {self.process_message (message);}
      }
    }
  }
  
  fn process_message (&mut self, message: MessageToRenderThread) {
    match message {
      MessageToRenderThread::PlaybackReachedTime (playback_time) => {
        self.latest_playback_reached = playback_time;
      },
      MessageToRenderThread::RestartPlaybackAt (script_time) => {
        self.stall(script_time);
      },
      MessageToRenderThread::ReplaceScript (script) => {
        self.script = script;
        self.stall (self.specified_playback_start_script_time.map (| script_start | {
          if let Some(frame_time) = self.actual_playback_start_frame_time {
            let duration = (self.latest_playback_reached - frame_time) as f64/SAMPLE_HZ as f64;
            script_start + duration
          }
          else {
            script_start
          }
        }));
      },
    }
  }
  
  fn stall(&mut self, stall_time: Option<f64>) {
    self.render_queue.clear();
    self.playback_queue.clear();
    self.actual_playback_start_frame_time = None;
    self.specified_playback_start_script_time = stall_time;
    self.renders_queued_until = 0.0;
    self.portaudio_sender.send (MessageToPortaudioThread::Stop).unwrap();
    //self.main_sender.send (MessageToFrontend::PlaybackStalledAt(stall_time)).unwrap();
    if let Some(stall_time) = stall_time {
      for note in & self.script.notes {
        let duration_from_start = note.start_time - stall_time;
        if duration_from_start < 0.0 && note.end_time() > stall_time {
          self.render_queue.push (RenderPreparingNote {note: note.clone(), duration_from_start});
        }
      }
    }
  }
  
  fn render_step (&mut self)-> bool {
    let start_script_time = match self.specified_playback_start_script_time {
      Some (a) => a,
      None => return false
    };
    if let Some(next) = self.render_queue.pop() {
      note_frames(& next.note);
      self.playback_queue.push(next);
      return true;
    }
    
    let next_scheduled_termination: Option<FrameTime> = None;
    
    match self.actual_playback_start_frame_time {
      Some(frame_start) => {
        if let Some(next) = self.playback_queue.pop() {
          let note_frame_time = frame_start + (next.duration_from_start * SAMPLE_HZ).round() as FrameTime;
          if note_frame_time > self.latest_playback_reached + FRAMES_PER_BUFFER as FrameTime {
            //eprintln!(" Sending frames {:?} ", note_frames (& next.note).len());
            self.portaudio_sender.send (MessageToPortaudioThread::AddNote(PlaybackNote {
              frames: note_frames(& next.note),
              first_sample_time: note_frame_time,
              
              fadein_start: if note_frame_time < frame_start { Some(frame_start) } else { None },
              fadeout_end: next_scheduled_termination.and_then (| termination | if termination < note_frame_time + note_frames(& next.note).len() as FrameTime {Some (termination)} else {None}),
            })).unwrap();
          }
          else {
            //self.playback_queue.push (next);
            self.stall(Some(next.note.start_time));
          }
          return true;
        }
        if self.renders_queued_until > (self.latest_playback_reached - frame_start) as f64 / SAMPLE_HZ as f64 + 5.0 {
          return false;
        }
      }
      None => {
        if self.renders_queued_until > start_script_time + 0.1 {
          self.actual_playback_start_frame_time = Some(self.latest_playback_reached + FRAMES_PER_BUFFER as FrameTime*2);
          //self.main_sender.send (MessageToFrontend::PlaybackResumed).unwrap();
        }
      }
    }
    
    let queue_until = self.renders_queued_until + 0.1;
    for note in & self.script.notes {
      let duration_from_start = note.start_time - start_script_time;
      if !(duration_from_start < self.renders_queued_until) && duration_from_start < queue_until {
        self.render_queue.push (RenderPreparingNote {note: note.clone(), duration_from_start});
      }
    }
    self.renders_queued_until = queue_until;
    return true;
  }
}


struct RenderThreadHandle {
  
}


pub fn spawn_render_thread() -> Sender<MessageToRenderThread> {
  //println!("Hello from backend (stdout)");
  eprintln!("Hello from backend (stderr)");

  
  let (send_to_render_thread, receive_on_render_thread) = channel();
  let (send_to_portaudio_thread, receive_on_portaudio_thread) = channel();
  
  let mut portaudio_thread = PortaudioThread {
    notes: Vec::new(),
    next_frame_time: 0,
    receiver: receive_on_portaudio_thread,
    sender: send_to_render_thread.clone(),
  };
  
  let mut render_thread = RenderThread {
    receiver: receive_on_render_thread,
    portaudio_sender: send_to_portaudio_thread,
    //main_sender: send_to_frontend,
    script: PlaybackScript::default(),
    render_queue: Vec::new(),
    playback_queue: Vec::new(),
    latest_playback_reached: 0,
    renders_queued_until: 0.0,
    specified_playback_start_script_time: None,
    actual_playback_start_frame_time: None,
  };
  
  
  thread::spawn(move || {



  let pa = portaudio::PortAudio::new().unwrap();
  //eprintln!("def output: {:?}", (pa.default_output_device(), pa.device_info(pa.default_output_device().unwrap())));
  /*let mut foo = "".to_string();
  for device in pa.devices().unwrap() {
    let (idx, info) = device.unwrap();
    foo = format!("{}{:?}\n", foo, (idx, info.name, info.max_input_channels, info.max_output_channels));
  }
  eprintln!("All devices: {}", foo);*/
  
  let settings = pa.default_output_stream_settings::<Output>(
    CHANNELS as i32,
    SAMPLE_HZ,
    FRAMES_PER_BUFFER as u32,
  ).unwrap();
  /*let device = portaudio::DeviceIndex(4);
  let settings = portaudio::OutputStreamSettings::new(
    portaudio::StreamParameters::new(
      device, CHANNELS as i32, true, pa.device_info(device).unwrap().default_low_output_latency
    ),
    SAMPLE_HZ, FRAMES_PER_BUFFER as u32
  );*/
  let mut stream = pa.open_non_blocking_stream(settings, move |p| portaudio_thread.call(p)).unwrap();
  stream.start().unwrap();
    render_thread.render_loop();
  });
  
  send_to_render_thread
}


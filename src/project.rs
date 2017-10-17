use super::*;

use dsp::sample::ToFrameSliceMut;
use dsp::Frame;
use portaudio::{PortAudio, Stream, NonBlocking, Flow, StreamSettings, OutputStreamSettings};
use std::sync::atomic::{Ordering, AtomicI32};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::time::Duration;
use notify::{self, Watcher, RecommendedWatcher, RecursiveMode, DebouncedEvent};

use phrase::Phrase;

type Output = f32;
const CHANNELS: usize = 2;



#[derive (Serialize)]
pub struct GuiPhrase {
  pub data: Phrase,
  pub timed_with_playback: bool,
  pub editable: bool,
}

#[derive (Serialize)]
pub enum GuiUpdate {
  ReplacePhrase (String, GuiPhrase),
  UpdatePlaybackPosition (NoteTime),
}

#[derive (Deserialize)]
pub enum GuiInput {
  SetPlaybackRange (NoteTime, NoteTime),
  EditPhrase (String, Phrase),
}


pub fn watch_phrases<F: FnMut(&HashMap<String, Phrase>, &HashSet<String>)> (project_path: &Path, changed_callback: F) {
  let phrases_path = project_path.join("editable/phrases");
  let (sender, receiver) = channel();
  let mut watcher = notify::watcher (sender, Duration::from_millis(100)).unwrap();
  watcher.watch (phrases_path, RecursiveMode::Recursive);
  
  let mut phrases = HashMap::new();
  let mut changed = HashSet::new();
  
    let handle_path = |path: PathBuf| {
      let name = match path.file_stem() {
        None=> {
          printlnerr!("Error during codecophony::project::watch_phrases: Couldn't get file_stem of path: {:?}", path);
          return;
        }
        Some(a)=>match a.to_str() {
        Some(a)=> String::from_str(a).unwrap(),
        None=> {
          printlnerr!("Error during codecophony::project::watch_phrases: Couldn't convert phrase path to string: {:?}", path);
          return;
        }
      }};
      
      changed.insert (name.clone());
     
      let mut file = match File::open (path) {
        Ok(a) => a,
        Err(NotFound) => {
          phrases.remove(&name);
          return;
        },
        Err(e) => {
          printlnerr!("File error during codecophony::project::watch_phrases: {:?}", e);
          return;
        }
      };
      let phrase = match serde_json::from_reader (file) {
        Ok(a) => a,
        Err(e) => {
          printlnerr!("Error parsing JSON during codecophony::project::watch_phrases: {:?}", e);
          return;
        }
      };
      phrases.insert (name, phrase);
    };

  for entry in ::std::fs::read_dir(phrases_path).unwrap() {
    handle_path (entry.unwrap().path());
  }
  
  loop {
    changed_callback (& phrases, & changed);
    changed.clear();
  
    let mut event = receiver.recv().unwrap();
    
    loop {
      match event {
        DebouncedEvent::Write(path) => handle_path(path),
        DebouncedEvent::Create(path) => handle_path(path),
        DebouncedEvent::Remove(path) => handle_path(path),
        DebouncedEvent::Rename(first, second) => { handle_path(first); handle_path (second);},
        _=>(),
      };
      if let Ok(a) = receiver.try_recv() {
        event = a;
      }
      else {
        break;
      }
    }
  }
}
    
pub fn write_phrase (project_path: &Path, name: &str, phrase: &Phrase) {
  let phrases_path = project_path.join("generated/phrases");
  let phrase_path = phrases_path.join(name).join(".json");
  let mut file = match File::create (phrase_path) {
    Ok(a) => a,
    Err(e) => {
      printlnerr!("File error during codecophony::project::write_phrase: {:?}", e);
      return;
    }
  };
  match serde_json::to_writer_pretty (file, phrase) {
    Ok(_) => (),
    Err(e) => {
      printlnerr!("Error while writing JSON during codecophony::project::write_phrase: {:?}", e);
    }
  };
}

pub fn set_playback_data (project_path: &Path, sample_hz: f64, data: Option<Box<Renderable<[Output; CHANNELS]> + Send>>) {
  let mut guard = GLOBALS.lock().unwrap();
  if guard.as_ref().map_or(true, |globals| globals.sample_hz != sample_hz || globals.project_path != project_path) {
    *guard = None;
    *guard = Some(Globals::new(project_path, sample_hz));
  }
  guard.as_mut().unwrap().set_playback_data (data);
}
  

#[derive (Default)]
struct SharedPlaybackData {
  playback_data: Mutex<Option<Box<Renderable<[Output; CHANNELS]> + Send>>>,
  playback_position: AtomicI32,
  playback_start: AtomicI32,
  playback_end: AtomicI32,
}

struct Globals {
  sample_hz: f64,
  project_path: PathBuf,
  watcher: notify::RecommendedWatcher,
  pa: PortAudio,
  stream: Stream <NonBlocking, <OutputStreamSettings<Output> as StreamSettings>::Flow>,
  inner: Arc<SharedPlaybackData>,
}

lazy_static! {
  static ref GLOBALS: Mutex<Option<Globals>> = Mutex::new(None);
}

impl Globals {
  fn new(project_path: &Path, sample_hz: f64) -> Globals {
    let inner = Arc::new(SharedPlaybackData {
      playback_start: AtomicI32::new(0),
      playback_end: AtomicI32::new(10*(sample_hz as FrameTime)),
      playback_position: AtomicI32::new(0),
      playback_data: Default::default(),
    });
    let callback_inner = inner.clone();
    let callback = move |portaudio::OutputStreamCallbackArgs { buffer, .. }| {
      let buffer: &mut [[Output; CHANNELS]] = buffer.to_frame_slice_mut().unwrap();
      dsp::slice::equilibrium(buffer);
      
      if let Some(note) = callback_inner.playback_data.lock().unwrap().as_ref() {
        let position = callback_inner.playback_position.load(Ordering::Relaxed);
        let start = callback_inner.playback_start.load(Ordering::Relaxed);
        let end = callback_inner.playback_end.load(Ordering::Relaxed);
        Renderable::<[Output; CHANNELS]>::render(&**note, buffer, position, sample_hz);
        
        let mut new_position = position + buffer.len() as i32;
        if new_position < start || new_position > end {new_position = start;}
        callback_inner.playback_position.compare_and_swap(position, new_position, Ordering::Relaxed);
      }
      
      portaudio::Continue
    };
  
    let pa = portaudio::PortAudio::new().unwrap();
    let settings = pa.default_output_stream_settings::<Output>(
      CHANNELS as i32,
      sample_hz,
      4096, // frames per buffer
    ).unwrap();
    let mut stream = pa.open_non_blocking_stream(settings, callback).unwrap();
    stream.start().unwrap();
    
    let ui_path = project_path.join ("ui/playback.json");
    
    
    Globals {
      sample_hz,
      pa,
      stream,
      inner,
      project_path: project_path.to_path_buf(),
      watcher: watcher,
    }
  }
  
  fn set_playback_data (&self, data: Option<Box<Renderable<[Output; CHANNELS]> + Send>>) {
    (*self.inner.playback_data.lock().unwrap()) = data;
  }
  fn set_playback_range (&self, range: (i32,i32)) {
    self.inner.playback_start.store(range.0, Ordering::Relaxed);
    self.inner.playback_end  .store(range.1, Ordering::Relaxed);
  }
  fn gui_updates(&self)->Vec<GuiUpdate> {
    vec![GuiUpdate::UpdatePlaybackPosition(self.inner.playback_position.load(Ordering::Relaxed) as NoteTime / self.sample_hz)]
  }
  fn apply_gui_input(&mut self, input: &GuiInput) {
    match input {
      &GuiInput::SetPlaybackRange (start,end) => {
        self.set_playback_range (((start*self.sample_hz) as FrameTime, (end*self.sample_hz) as FrameTime));
      },
      _=>(),
    }
  }
}

impl Drop for Globals {
  fn drop(&mut self) {
    self.stream.stop();
  }
}


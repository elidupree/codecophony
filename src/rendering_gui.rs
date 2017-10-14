use super::*;

use dsp::sample::ToFrameSliceMut;
use dsp::Frame;
use portaudio::{PortAudio, Stream, NonBlocking, Flow, StreamSettings, OutputStreamSettings};
use std::sync::atomic::{Ordering, AtomicI32};
use std::sync::{Arc, Mutex};

use phrase::Phrase;

type Output = f32;
const CHANNELS: usize = 2;

#[derive (Default)]
pub struct RenderingGuiInner {
  playback_data: Mutex<Option<Box<Renderable<[Output; CHANNELS]> + Send>>>,
  playback_position: AtomicI32,
  playback_start: AtomicI32,
  playback_end: AtomicI32,
}

pub struct RenderingGui {
  sample_hz: f64,
  pa: PortAudio,
  stream: Stream <NonBlocking, <OutputStreamSettings<Output> as StreamSettings>::Flow>,
  inner: Arc<RenderingGuiInner>,
  visuals: NonSharedVisuallyRenderedStuff,
}

#[derive (Clone, Serialize, Default)]
pub struct NonSharedVisuallyRenderedStuff {
  pub phrases: Vec<Phrase>,
}

#[derive (Serialize)]
pub struct VisuallyRenderedStuff<'a> {
  pub music_data: &'a NonSharedVisuallyRenderedStuff,
  pub playback_position: i32,
  pub playback_start: i32,
  pub playback_end: i32,
}

impl RenderingGui {

  pub fn new(sample_hz: f64) -> RenderingGui {
    let inner = Arc::new(RenderingGuiInner {
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
        if new_position > end {new_position = start;}
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
    RenderingGui {
      sample_hz,
      pa,
      stream,
      inner,
      visuals: Default::default(),
    }
  }
  
  pub fn set_playback_data (&self, data: Option<Box<Renderable<[Output; CHANNELS]> + Send>>) {
    (*self.inner.playback_data.lock().unwrap()) = data;
  }
  pub fn set_playback_range (&self, range: (i32,i32)) {
    self.inner.playback_start.store(range.0, Ordering::Relaxed);
    self.inner.playback_end  .store(range.1, Ordering::Relaxed);
  }
  pub fn set_visuals (&mut self, visuals: NonSharedVisuallyRenderedStuff) {
    self.visuals = visuals;
  }
  pub fn export_visuals (&self)->String {
    serde_json::to_string(&VisuallyRenderedStuff {
      music_data: &self.visuals,
      playback_position: self.inner.playback_position.load(Ordering::Relaxed),
      playback_start   : self.inner.playback_start   .load(Ordering::Relaxed),
      playback_end     : self.inner.playback_end     .load(Ordering::Relaxed),
    }).unwrap()
  }
}

impl Drop for RenderingGui {
  fn drop(&mut self) {
    self.stream.stop();
  }
}

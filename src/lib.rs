#![feature (specialization, iterator_step_by, integer_atomics)]

extern crate rand;
extern crate fluidsynth;
extern crate hound;
extern crate dsp;
extern crate ordered_float;
#[macro_use]
extern crate lazy_static;
extern crate portaudio;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

macro_rules! printlnerr(
    ($($arg:tt)*) => { {use std::io::Write;
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

use std::cmp::{min, max};
use std::collections::HashMap;
use std::str::FromStr;
use std::cell::RefCell;
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::iter::{self, FromIterator};

use dsp::Sample;
use ordered_float::{NotNaN, OrderedFloat};


pub mod rendering_gui;
pub mod phrase;


pub type FrameTime = i32;
pub type NoteTime = f64;
pub type Semitones = i32;
pub const SEMITONE_RATIO: f64 = 1.0594631f64;
pub const JUST_NOTICEABLE_FREQUENCY_RATIO: f64 = 1.006f64;


pub trait Windowed {
  fn start (&self)->NoteTime;
  fn end (&self)->NoteTime;
}
pub trait Renderable<Frame: dsp::Frame>: Windowed {
  fn render (&self, buffer: &mut [Frame], start: FrameTime, sample_hz: f64);
}

macro_rules! impl_windowed_for_iterable {
  ($self_hack:ident, $iter: expr) => {
    fn start (&$self_hack)->NoteTime {
      match $iter.map (| note | OrderedFloat(note.start())).min() {
        None => 1.0,
        Some(a)=>a.0,
      }
    }
    fn end (&$self_hack)->NoteTime {
      match $iter.map (| note | OrderedFloat(note.end())).max() {
        None => 0.0,
        Some(a)=>a.0,
      }
    }
  };
}
macro_rules! impl_renderable_for_iterable {
  ($self_hack:ident, $iter: expr) => {
    fn render(&$self_hack, buffer: &mut [Frame], start: FrameTime, sample_hz: f64) { 
      for note in $iter {
        let afterend = start + buffer.len() as FrameTime;
        let note_start = max(start, (note.start()*sample_hz).ceil() as FrameTime);
        let note_afterend = min(afterend, (note.end()*sample_hz).floor() as FrameTime + 1);
        if note_afterend > note_start {
          note.render(&mut buffer[(note_start-start) as usize .. (note_afterend-start) as usize], note_start, sample_hz);
        }
      }
    }
  };
}

impl<T: Windowed> Windowed for Vec<T> {
  impl_windowed_for_iterable!(self, self.iter());
}

impl<Frame: dsp::Frame, T: Renderable<Frame>> Renderable<Frame> for Vec<T> {
  impl_renderable_for_iterable!(self, self.iter());
}


macro_rules! impl_windowed_for_derefable {
  () => {
    fn start (&self)->NoteTime {
      (**self).start()
    }
    fn end (&self)->NoteTime {
      (**self).end()
    }
  };
}
macro_rules! impl_renderable_for_derefable {
  () => {
    fn render(&self, buffer: &mut [Frame], start: FrameTime, sample_hz: f64) { 
      (**self).render(buffer, start, sample_hz);
    }
  };
}

impl<T: Windowed + ?Sized> Windowed for Box<T> {
  impl_windowed_for_derefable!();
}

impl<Frame: dsp::Frame, T: Renderable<Frame> + ?Sized> Renderable<Frame> for Box<T> {
  impl_renderable_for_derefable!();
}


pub trait Nudgable {
  fn nudge(&mut self, distance: NoteTime);
}

pub trait Dilatable {
  fn dilate(&mut self, amount: f64, origin: f64);
}

pub trait Pitched {
  fn frequency(&self)->f64;
}

pub trait Transposable {
  fn transpose(&mut self, amount: Semitones);
}

pub trait PitchShiftable {
  fn pitch_shift(&mut self, frequency_ratio: f64);
}

impl <T: PitchShiftable> Transposable for T {
  default fn transpose(&mut self, amount: Semitones) {
    self.pitch_shift(SEMITONE_RATIO.powi(amount));
  }
}


#[derive (Clone)]
pub struct PositionedSequence<Frame: dsp::Frame, Frames: Borrow<[Frame]>> {
  pub start: FrameTime,
  pub sample_hz: f64,
  pub frames: Frames,
  _marker: PhantomData<Frame>,
}
impl<Frame: dsp::Frame, Frames: Borrow<[Frame]>> Windowed for PositionedSequence<Frame, Frames> {
  fn start (&self)->NoteTime {self.start as NoteTime / self.sample_hz}
  fn end (&self)->NoteTime {(self.start + self.frames.borrow().len() as FrameTime-1) as NoteTime / self.sample_hz}
}
impl<Frame: dsp::Frame, Frames: Borrow<[Frame]>> Renderable<Frame> for PositionedSequence<Frame, Frames>
  where <Frame::Sample as Sample>::Float: dsp::FromSample<f64> {
  fn render(&self, buffer: &mut [Frame], start: FrameTime, sample_hz: f64) {
    if sample_hz == self.sample_hz {
      for (index, value_mut) in buffer.iter_mut().enumerate() {
        let my_index = (start + index as FrameTime - self.start) as usize;
        *value_mut = value_mut.add_amp(self.frames.borrow().get(my_index).cloned().unwrap_or(Frame::equilibrium()).to_signed_frame());
      }
    }
    else {
      // if the sample rates are different, resample it
      for (index, value_mut) in buffer.iter_mut().enumerate() {
        let time = (start as f64 + index as f64) * sample_hz;
        *value_mut = value_mut.add_amp(self.interpolate_sample (time).to_signed_frame());
      }
    }
  }
}

impl<Frame: dsp::Frame, Frames: Borrow<[Frame]>> Nudgable for PositionedSequence<Frame, Frames>
{
  fn nudge(&mut self, distance: NoteTime) {
    // The distance may not be an exact multiple of the frame time. 
    // By default, it seems better to slightly misplace the resulting data than to resample it.
    self.start += (distance*self.sample_hz).round() as FrameTime;
  }
}

impl<Frame: dsp::Frame, Frames: Borrow<[Frame]>> PositionedSequence<Frame, Frames>
  where <Frame::Sample as Sample>::Float: dsp::FromSample<f64> {
  /// do some boring old linear resampling.
  pub fn interpolate_sample (&self, time: f64) -> Frame {
    let relative_time = time*self.sample_hz - self.start as f64;
    let previous_index = relative_time.trunc() as usize;
    let previous = self.frames.borrow().get (previous_index).cloned().unwrap_or (Frame::equilibrium());
    let next = self.frames.borrow().get (previous_index.wrapping_add(1)).cloned().unwrap_or (Frame::equilibrium());
    let factor = relative_time.fract();
    previous.scale_amp(Sample::from_sample(1.0-factor)).add_amp(next.scale_amp(Sample::from_sample(factor)).to_signed_frame())
  }
}

impl<Frame: dsp::Frame, Frames: Borrow<[Frame]>> PositionedSequence<Frame, Frames>
  where Frames: FromIterator<Frame> + BorrowMut<[Frame]> {
  pub fn rendered_from <N: Renderable<Frame> + ?Sized> (note: &N, sample_hz: f64)->Self {
    let earliest = (note.start()*sample_hz).ceil() as FrameTime;
    let latest = (note.end()*sample_hz).floor() as FrameTime;
    let length = max(0,latest+1-earliest) as usize;
    let mut frames: Frames = iter::repeat(Frame::equilibrium()).take(length).collect();
    note.render(frames.borrow_mut(), earliest, sample_hz);

    PositionedSequence {
      start: earliest,
      sample_hz,
      frames,
      _marker: PhantomData,
    }
  }
}

  

#[derive (Clone, Debug)]
pub struct SineWave {
  pub start: NoteTime,
  pub duration: NoteTime,
  pub frequency: f64,
  pub amplitude: f64,
}

impl SineWave {
  fn value(&self, time: NoteTime)->NoteTime {
    let start = self.start;
    let end = self.end();
    if time < start || time > end { return 0.0; }
    let envelope_time = if self.duration<1.0 {self.duration*0.05} else {0.05};
    let envelope =
      if time < start + envelope_time {
        (time-start) / envelope_time
      }
      else if time > end - envelope_time {
        (end-time) / envelope_time
      }
      else {1.0};
    //printlnerr!("{:?}", envelope);
    self.amplitude * envelope * (self.frequency * time * (std::f64::consts::PI * 2.0)).sin()
  }
}

impl Windowed for SineWave {
  fn start (&self)->NoteTime {self.start}
  fn end (&self)->NoteTime {self.start+self.duration}
}
impl<Frame: dsp::Frame> Renderable<Frame> for SineWave
    where Frame::Sample: dsp::FromSample<f64> {
  fn render(&self, buffer: &mut [Frame], start: FrameTime, sample_hz: f64) {
    for (index, value_mut) in buffer.iter_mut().enumerate() {
      let time = (start + index as FrameTime) as f64/sample_hz;
      let value = Frame::Sample::from_sample(self.value (time));
      *value_mut = value_mut.add_amp(Frame::from_fn(|_| value).to_signed_frame());
    }
  }
}

impl Nudgable for SineWave {
  fn nudge(&mut self, distance: NoteTime) {
    self.start += distance;
  }
}

impl Dilatable for SineWave {
  fn dilate(&mut self, amount: f64, origin: f64) {
    self.start = origin + (self.start-origin)*amount;
    self.duration *= amount;
  }
}

impl Pitched for SineWave {
  fn frequency(&self)->f64 {self.frequency}
}

impl PitchShiftable for SineWave {
  fn pitch_shift(&mut self, frequency_ratio: f64) {
    self.frequency *= frequency_ratio;
  }
}


#[derive (Clone, Debug)]
pub struct MIDIPitched;
#[derive (Clone, Debug)]
pub struct MIDIPercussion;

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct FluidsynthDirectlyRenderableMIDIInstrument {
  channel: i32,
  bank: u32,
  preset: u32,
}
const PERCUSSION_CHANNEL: i32 = 9;
impl FluidsynthDirectlyRenderableMIDIInstrument {
  // offsets the program by one to use the same numbers as the General MIDI specification, which numbers the instruments from one rather than 0
  fn pitched(program: u32) -> Self {
    FluidsynthDirectlyRenderableMIDIInstrument {
      bank: 0,
      preset: program - 1,
      channel: 0,
    }
  }
  fn percussion() -> Self {
    FluidsynthDirectlyRenderableMIDIInstrument {
      bank: 0,
      preset: 0,
      channel: PERCUSSION_CHANNEL,
    }
  }
  pub fn is_percussion(&self) -> bool {
    self.channel == PERCUSSION_CHANNEL
  }
}

#[derive (Clone, PartialEq, Eq, Hash, Debug)]
pub struct FluidsynthDirectlyRenderableMIDINote {
  pub duration: NotNaN<NoteTime>,
  pub pitch: i32,
  //pub pitch_bend: i32,
  pub velocity: i32,
  pub instrument: FluidsynthDirectlyRenderableMIDIInstrument,
}

#[derive (Clone, Debug)]
pub struct MIDINote<PitchedOrPercussion> {
  start: NoteTime,
  raw: FluidsynthDirectlyRenderableMIDINote,
  _marker: PhantomData<PitchedOrPercussion>,
}

pub type MIDIPitchedNote = MIDINote<MIDIPitched>;
pub type MIDIPercussionNote = MIDINote<MIDIPercussion>;

impl MIDINote<MIDIPitched> {
  pub fn new(start: f64, duration: f64, pitch: i32, velocity: i32, instrument: u32)->Self {
    MIDINote {
      start,
      raw: FluidsynthDirectlyRenderableMIDINote {
        duration: NotNaN::new (duration).unwrap(),
        pitch, velocity,
        instrument: FluidsynthDirectlyRenderableMIDIInstrument::pitched(instrument),
      },
      _marker: PhantomData,
    }
  }
}
impl MIDINote<MIDIPercussion> {
  pub fn new(start: f64, duration: f64, velocity: i32, instrument: i32)->Self {
    MIDINote {
      start,
      raw: FluidsynthDirectlyRenderableMIDINote {
        duration: NotNaN::new (duration).unwrap(),
        pitch: instrument, velocity,
        instrument: FluidsynthDirectlyRenderableMIDIInstrument::percussion(),
      },
      _marker: PhantomData,
    }
  }
}

impl<PitchedOrPercussion> Nudgable for MIDINote<PitchedOrPercussion> {
  fn nudge(&mut self, distance: NoteTime) {
    self.start += distance;
  }
}

impl<PitchedOrPercussion> Dilatable for MIDINote<PitchedOrPercussion> {
  fn dilate(&mut self, amount: f64, origin: f64) {
    self.start = origin + (self.start-origin)*amount;
    self.raw.duration *= amount;
  }
}

pub fn midi_pitch_to_frequency(pitch: i32)->f64 {
  440.0*SEMITONE_RATIO.powi(pitch-69)
}
pub fn frequency_to_nearest_midi_pitch(frequency: f64)->i32 {
  ((frequency/440.0).ln()/SEMITONE_RATIO.ln()).round() as i32 + 69
}

impl Pitched for MIDINote<MIDIPitched> {
  fn frequency(&self)->f64 {
    midi_pitch_to_frequency(self.raw.pitch)
  }
}

impl Transposable for MIDINote<MIDIPitched> {
  fn transpose(&mut self, amount: Semitones) {
    self.raw.pitch += amount as i32;
  }
}


struct Fluid {
  settings: fluidsynth::settings::Settings,
  synth: fluidsynth::synth::Synth,
  font_id: u32,
  notes: HashMap<FluidsynthDirectlyRenderableMIDINote, [Vec<f32>;2]>,
}
thread_local! {
  static SYNTHESIZERS: RefCell<HashMap<NotNaN<f64>, Fluid>> = RefCell::new (HashMap::new());
}
fn with_fluid <Return, F: FnOnce (&mut Fluid)->Return> (sample_hz: f64, callback: F)->Return {
  SYNTHESIZERS.with (move | synthesizers | {
    let mut guard = synthesizers.borrow_mut();
    let mut synthesizer = guard.entry (NotNaN::new(sample_hz).unwrap()).or_insert_with (move | | {
      let mut settings = fluidsynth::settings::Settings::new();
      settings.setnum("synth.sample-rate", sample_hz);
      settings.setnum("synth.gain", 1.0);
      let mut synthesizer = fluidsynth::synth::Synth::new(&mut settings);
      let font_id = synthesizer.sfload("/usr/share/sounds/sf2/FluidR3_GM.sf2", 1).unwrap();
      Fluid {settings: settings, synth: synthesizer, font_id: font_id, notes: HashMap::new()}
    });
    
    callback (synthesizer)
  })
}

fn with_rendered_midi_note <Return, F: FnOnce (&[Vec<f32>;2])->Return> (note: &FluidsynthDirectlyRenderableMIDINote, sample_hz: f64, callback: F) {
  with_fluid (sample_hz, | fluid | {
    let channels = {
      let synth = &mut fluid.synth;
      let font_id = fluid.font_id;
      fluid.notes.entry (note.clone()).or_insert_with(|| {
        if !note.instrument.is_percussion() {
          synth.program_select(note.instrument.channel, font_id,
                                            note.instrument.bank,
                                            note.instrument.preset);
        }
        synth.noteon(note.instrument.channel, note.pitch, note.velocity);
        let mut left = Vec::new();
        let mut right = Vec::new();
        assert! (synth.write_f32 ((note.duration.into_inner()*sample_hz) as usize, &mut left, &mut right));
        if !note.instrument.is_percussion() {
          synth.noteoff(note.instrument.channel, note.pitch);
        }
        for index in 0..1000 {
          let duration =(1.0+sample_hz/10.0) as usize;
          assert! (synth.write_f32 (duration, &mut left, &mut right));
          // continue rendering until we observe silence
          if left.iter().rev().take (duration).chain (right.iter().rev().take (duration)).all(| sample | (sample.abs() < 0.000001)) {
            break;
          }
          assert!(index <900);
        }
        [left, right]
      })
    };
    
    callback(channels);
  });
}

impl<PitchedOrPercussion> Windowed for MIDINote<PitchedOrPercussion> {
  fn start (&self)->NoteTime {self.start}
  fn end (&self)->NoteTime {self.start+self.raw.duration.into_inner()+5.0}
}
impl<PitchedOrPercussion, Frame: dsp::Frame> Renderable<Frame> for MIDINote<PitchedOrPercussion> 
    where Frame::Sample: dsp::FromSample<f32> {
  fn render(&self, buffer: &mut [Frame], start: FrameTime, sample_hz: f64) {
    with_rendered_midi_note (&self.raw, sample_hz, | channels | {
      let rounded_note_start = (self.start*sample_hz).round() as FrameTime;
      for (index, value_mut) in buffer.iter_mut().enumerate() {
        let rendered_index = ((index as FrameTime + start) - rounded_note_start) as usize;
        let value = Frame::Sample::from_sample(if let Some(left) = channels[0].get(rendered_index) {
          let right = channels[1].get(rendered_index).unwrap();
          assert!(left.is_finite());
          assert!(right.is_finite());
          // hack: convert stereo to mono
          (left + right)*0.5
        }
        else {
          0.0
        });
        *value_mut = value_mut.add_amp(Frame::from_fn(|_| value).to_signed_frame());
      }
    })
  }
}

pub fn enforce_maximum<Frame: dsp::Frame<Sample = i32>>(sequence: &mut [Frame], forced_maximum: i32) {
  let maximum = match sequence.iter().flat_map (| frame | frame.channels()).map (| sample | sample.abs()).max() {
    None => return,
    Some(a) => a,
  };
  if maximum <= forced_maximum {
    return;
  }
  for frame in sequence.iter_mut() {
    *frame = frame.map(|sample| (sample * forced_maximum * 2 + maximum) / maximum*2);
  }
}

pub mod interval_optimizer; 


// trait Interpreter

/*
struct BasicInterpreter<Render: Renderer> {
  notes: Vec<Render>,
  now: f64,
  step_size: f64,
  sustained_notes: HashMap<Semitones, Note<Render>>,
  latest_notes: HashMap<Semitones, Note<Render>>,
  command_in_progress: Option<String>,
}



trait InterpreterCaller <Render: Renderer> {
  fn create(&mut self, semitones: Semitones) -> Render;
}
impl<Render: Renderer> Default for BasicInterpreter<Render> {
  fn default() -> Self {
    BasicInterpreter::<Render> {
      now: 0.0,
      step_size: 1.0,
      notes: Default::default(),
      sustained_notes: Default::default(),
      latest_notes: Default::default(),
      command_in_progress: None,
    }
  }
}
impl<Render: Renderer> BasicInterpreter<Render> {
  fn finish_note(&mut self, note: Note<Render>) {
    let mut note = note;
    note.basics.duration = self.now - note.basics.start;//note.set_end (self.now);
    self.notes.push(note);
  }
  fn finish_notes(&mut self) {
    let last_begin = self.latest_notes.values().fold(-900000000.0f64, |max, note: &Note<Render>| {
      if note.basics.start > max {
        note.basics.start
      } else {
        max
      }
    });
    let step_end = last_begin + self.step_size;
    if step_end > self.now {
      self.now = step_end
    };

    for (_, note) in self.latest_notes.clone().iter() {
      self.finish_note(note.clone());
    }
    self.latest_notes.clear();

  }

  fn create_note<Caller: InterpreterCaller<Render>>(&mut self,
                                                    caller: &mut Caller,
                                                    semitones: Semitones)
                                                    -> Note<Render> {
    Note::<Render> {
      basics: NoteBasics {
        start: self.now,
        duration: 0.0,
      },
      renderer: caller.create(semitones),
    }
  }

  fn interpret<Caller: InterpreterCaller<Render>>(&mut self, caller: &mut Caller, command: &str) {
    match self.command_in_progress.clone() {

      None => {
        match Semitones::from_str(command) {
          Ok(semitones) => {
            self.finish_notes();
            let note = self.create_note(caller, semitones);
            self.latest_notes.insert(semitones, note);
          }
          Err(_) => {
            match command {
              "finish" => self.finish_notes(),
              _ => self.command_in_progress = Some(command.to_string()),
            }
          }
        }
      }
      Some(last_command) => {
        match &*last_command {
          "and" => {
            let semitones = Semitones::from_str(command).unwrap();
            let note = self.create_note(caller, semitones);
            self.latest_notes.insert(semitones, note);
          }
          "sustain" => {
            let semitones = Semitones::from_str(command).unwrap();
            let note = self.create_note(caller, semitones);
            self.sustained_notes.insert(semitones, note);
          }
          "release" => {
            match Semitones::from_str(command) {
              Ok(semitones) => {
                let note = self.sustained_notes.remove(&semitones).unwrap();
                self.finish_note(note);
              }
              Err(_) => {
                for (_, note) in self.sustained_notes.clone().iter() {
                  self.finish_note(note.clone());
                }
                self.sustained_notes.clear();
              }
            }
          }
          "step" => {
            self.step_size = f64::from_str(command).unwrap();
          }
          "advance" => {
            let distance = f64::from_str(command).unwrap();
            assert!(distance >= 0.0);
            self.now += distance;
          }
          "at" => {
            let time = f64::from_str(command).unwrap();
            if time < self.now {
              self.finish_notes();
            }
            self.now = time;
          }
          _ => panic!(),
        };
        self.command_in_progress = None;
      }

    }
  }
}

struct MIDIInterpreter {
  prototype: MIDINote,
  velocity_adjustment: i16,
  command_in_progress: Option<String>,
}

impl InterpreterCaller<MIDINote> for MIDIInterpreter {
  fn create(&mut self, semitones: Semitones) -> MIDINote {
    let mut velocity = self.prototype.velocity;
    while self.velocity_adjustment > 0 {
      self.velocity_adjustment -= 1;
      velocity = (velocity * 2 + 128) / 3;
    }
    while self.velocity_adjustment < 0 {
      self.velocity_adjustment += 1;
      velocity = (velocity * 2) / 3;
    }
    MIDINote {
      pitch: self.prototype.pitch + semitones as i32,
      velocity: velocity,
      ..self.prototype.clone()
    }
  }
}

impl MIDIInterpreter {
  fn interpret(&mut self, basics: &mut BasicInterpreter<MIDINote>, command: &str) {
    match self.command_in_progress.clone() {
      None => {
        match command {
          "strong" => self.velocity_adjustment += 1,
          "quiet" => self.velocity_adjustment -= 1,
          "percussion" => self.prototype.instrument = MIDIInstrument::percussion(),
          parametric@"instrument" | parametric@"velocity" | parametric@"transpose" => {
            self.command_in_progress = Some(parametric.to_string())
          }
          other => basics.interpret(self, other),
        }
      }
      Some(last_command) => {
        match &*last_command {
          "instrument" => {
            self.prototype.instrument = MIDIInstrument::new(u32::from_str(command).unwrap())
          }
          "velocity" => self.prototype.velocity = i32::from_str(command).unwrap(),
          "transpose" => self.prototype.pitch = i32::from_str(command).unwrap(),
          _ => panic!(),
        };
        self.command_in_progress = None;
      }
    }
  }
}

pub fn scrawl_MIDI_notes(scrawl: &str) -> Vec<MIDINote> {
  let mut basics = BasicInterpreter::<MIDINote>::default();
  let mut specifics = MIDIInterpreter {
    velocity_adjustment: 0,
    prototype: MIDINote {
      pitch: 0,
      velocity: 64,
      instrument: MIDIInstrument::new(88),
    },
    command_in_progress: None,
  };
  for command in scrawl.split_whitespace() {
    specifics.interpret(&mut basics, command);
  }
  basics.finish_notes();
  basics.notes
}

*/

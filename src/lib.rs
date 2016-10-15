extern crate rand;
extern crate fluidsynth;
extern crate hound;

use std::cmp::{min, max};
use std::collections::HashMap;
use std::str::FromStr;
use std::cell::RefCell;


pub type Position = i32;
pub type Sample = i32;
pub type Semitones = i32;
pub const SEMITONE_RATIO: f64 = (1.0594631f64);

#[derive (Clone)]
pub struct Sequence {
  pub start: Position,
  pub samples: Vec<Sample>,
}

#[derive (Clone, Copy)]
pub struct NoteBasics {
  pub start: f64,
  pub duration: f64,
}

// trait SequenceTransform<RendererType: Renderer> : Clone + Fn (&mut Sequence, &Note <RendererType>)->() {}

#[derive (Clone)]
pub struct Note<RendererType: Renderer> {
  pub basics: NoteBasics,
  pub renderer: RendererType, // sequence_transforms: Vec<Box <SequenceTransform <RendererType> >>,
}

impl<RendererType: Renderer> Note<RendererType> {
  pub fn new(start: f64, duration: f64, renderer: RendererType) -> Note<RendererType> {
    Note::<RendererType> {
      basics: NoteBasics {
        start: start,
        duration: duration,
      },
      renderer: renderer, // , sequence_transforms: Vec::new (),
    }
  }
  pub fn render(&self, sample_rate: Position) -> Sequence {
    self.renderer.render(self.basics, sample_rate)
  }
}
impl<Render: Renderer + Transposable> Transposable for Note<Render> {
  fn transpose(&mut self, amount: Semitones) -> &mut Note<Render> {
    self.renderer.transpose(amount);
    self
  }
}
impl<Render: Renderer> Scalable for Note<Render> {
  fn scale_about(&mut self, amount: f64, origin: f64) -> &mut Note<Render> {
    self.basics.start = origin + (self.basics.start - origin) * amount;
    self.basics.duration *= amount;
    self
  }
}


#[derive (Clone)]
pub struct Notes<Render: Renderer> {
  data: Vec<Note<Render>>,
}
impl<Render: Renderer> std::ops::Deref for Notes<Render> {
  type Target =Vec< Note <Render>>;
  fn deref(&self) -> &Vec<Note<Render>> {
    &self.data
  }
}
impl<Render: Renderer> std::ops::DerefMut for Notes<Render> {
  fn deref_mut(&mut self) -> &mut Vec<Note<Render>> {
    &mut self.data
  }
}
impl<Render: Renderer> Default for Notes<Render> {
  fn default() -> Self {
    Notes::<Render> { data: Default::default() }
  }
}


impl<Render: Renderer> Notes<Render> {
  pub fn new() -> Notes<Render> {
    Notes::<Render> { data: Vec::new() }
  }
  pub fn add(&mut self, other: &Notes<Render>) {
    self.extend(other.iter().map(|note| note.clone()))
  }
  pub fn combining(parts: &[Self]) -> Self {
    let mut result = Self::new();
    for other in parts {
      result.add(other);
    }
    result
  }

  pub fn translate(&mut self, amount: f64) -> &mut Notes<Render> {
    for note in self.data.iter_mut() {
      note.basics.start += amount
    }
    self
  }
  pub fn translated(&self, amount: f64) -> Notes<Render> {
    let mut result = self.clone();
    result.translate(amount);
    result
  }
  pub fn modify_renderers(&mut self, modifier: &Fn(&mut Render)) -> &mut Notes<Render> {
    for note in self.data.iter_mut() {
      modifier(&mut note.renderer)
    }
    self
  }
  pub fn with_renderers(&self, modifier: &Fn(&mut Render)) -> Notes<Render> {
    let mut result = self.clone();
    result.modify_renderers(modifier);
    result
  }
}
impl<Render: Renderer + Transposable> Transposable for Notes<Render> {
  fn transpose(&mut self, amount: Semitones) -> &mut Notes<Render> {
    for note in self.data.iter_mut() {
      note.transpose(amount);
    }
    self
  }
}

impl<Render: Renderer> Scalable for Notes<Render> {
  fn scale_about(&mut self, amount: f64, origin: f64) -> &mut Notes<Render> {
    for note in self.data.iter_mut() {
      note.scale_about(amount, origin);
    }
    self
  }
}

impl<Render: Renderer> Renderer for Notes<Render> {
  fn render(&self, basics: NoteBasics, sample_rate: Position) -> Sequence {
    let sequences: Vec<Sequence> = self.iter().map(|note| note.render(sample_rate)).collect();
    let mut result = merge(&sequences);
    result.start += (basics.start * sample_rate as f64) as Position;
    result
  }
}


pub trait Renderer: Clone {
  fn render(&self, basics: NoteBasics, sample_rate: Position) -> Sequence;
  fn render_default(&self, sample_rate: Position) -> Sequence {
    self.render(NoteBasics {
                  start: 0.0,
                  duration: 0.0,
                },
                sample_rate)
  }
}

pub trait Transposable: Clone {
  fn transpose(&mut self, amount: Semitones) -> &mut Self;
  fn transposed(&self, amount: Semitones) -> Self {
    let mut result = self.clone();
    result.transpose(amount);
    result
  }
}
pub trait Scalable: Clone {
  fn scale(&mut self, amount: f64) -> &mut Self {
    self.scale_about(amount, 0.0)
  }
  fn scaled(&self, amount: f64) -> Self {
    let mut result = self.clone();
    result.scale(amount);
    result
  }
  fn scale_about(&mut self, amount: f64, origin: f64) -> &mut Self;
  fn scaled_about(&self, amount: f64, origin: f64) -> Self {
    let mut result = self.clone();
    result.scale_about(amount, origin);
    result
  }
}

#[derive (Clone)]
pub struct SineWave {
  frequency: f64,
  amplitude: f64,
}

impl Renderer for SineWave {
  fn render(&self, basics: NoteBasics, sample_rate: Position) -> Sequence {
    let mut samples: Vec<Sample> = Vec::new();
    let after = (basics.duration * sample_rate as f64) as Position;
    for time in 0..after {
      let mut sample = (self.amplitude *
                        (self.frequency * time as f64 * (std::f64::consts::PI * 2.0) /
                         sample_rate as f64)
                          .sin()) as Sample;
      if after - time < 20 {
        sample = sample * (after - time) / 20;
      }
      samples.push(sample);
    }
    Sequence {
      start: (basics.start * sample_rate as f64) as Position,
      samples: samples,
    }
  }
}
impl Transposable for SineWave {
  fn transpose(&mut self, amount: Semitones) -> &mut Self {
    self.frequency *= SEMITONE_RATIO.powi(amount);
    self
  }
}

#[derive (Clone)]
pub struct MIDIInstrument {
  channel: i32,
  bank: u32,
  preset: u32,
}
const PERCUSSION_CHANNEL: i32 = 9;
impl MIDIInstrument {
  // offsets the program by one to use the same numbers as the General MIDI specification, which numbers the instruments from one rather than 0
  pub fn new(program: u32) -> Self {
    MIDIInstrument {
      bank: 0,
      preset: program - 1,
      channel: 0,
    }
  }
  pub fn percussion() -> Self {
    MIDIInstrument {
      bank: 0,
      preset: 0,
      channel: PERCUSSION_CHANNEL,
    }
  }
  pub fn is_percussion(&self) -> bool {
    self.channel == PERCUSSION_CHANNEL
  }
}

#[derive (Clone)]
pub struct MIDINote {
  pub pitch: i32,
  pub velocity: i32,
  pub instrument: MIDIInstrument,
}
impl Transposable for MIDINote {
  fn transpose(&mut self, amount: Semitones) -> &mut Self {
    self.pitch += amount as i32;
    self
  }
}

struct Fluid {
  settings: fluidsynth::settings::Settings,
  synth: fluidsynth::synth::Synth,
  font_id: u32,
}
thread_local! {
  static SYNTHESIZERS: RefCell<HashMap<Position, Fluid>> = RefCell::new (HashMap::new());
}
fn with_fluid <Return, F: FnOnce (&mut Fluid)->Return> (sample_rate: Position, callback: F)->Return {
  SYNTHESIZERS.with (| synthesizers | {
    let mut guard = synthesizers.borrow_mut();
    let mut synthesizer = guard.entry (sample_rate).or_insert_with (| | {
      let mut settings = fluidsynth::settings::Settings::new();
      settings.setnum("synth.sample-rate", sample_rate as f64);
      settings.setnum("synth.gain", 1.0);
      let mut synthesizer = fluidsynth::synth::Synth::new(&mut settings);
      let font_id = synthesizer.sfload("/usr/share/sounds/sf2/FluidR3_GM.sf2", 1).unwrap();
      Fluid {settings: settings, synth: synthesizer, font_id: font_id}
    });
    
    callback (synthesizer)
  })
}

impl Renderer for MIDINote {
  fn render(&self, basics: NoteBasics, sample_rate: Position) -> Sequence {
    with_fluid (sample_rate, | fluid | {
      if !self.instrument.is_percussion() {
        fluid.synth.program_select(self.instrument.channel, fluid.font_id,
                                          self.instrument.bank,
                                          self.instrument.preset);
      }
      fluid.synth.noteon(self.instrument.channel, self.pitch, self.velocity);
      let mut left = Vec::new();
      let mut right = Vec::new();
      assert! (fluid.synth.write_f32 ((basics.duration*(sample_rate as f64)) as usize, &mut left, &mut right));
      if !self.instrument.is_percussion() {
        fluid.synth.noteoff(self.instrument.channel, self.pitch);
      }
      for index in 0..1000 {
        let duration =(1+sample_rate/10) as usize;
        assert! (fluid.synth.write_f32 (duration, &mut left, &mut right));
        // continue rendering until we observe silence
        if left.iter().rev().take (duration).chain (right.iter().rev().take (duration)).all(| sample | (sample.abs() < 0.000001)) {
          break;
        }
        assert!(index <900);
      }
      Sequence {
        start: (basics.start * sample_rate as f64) as Position,
        samples: left.into_iter().zip (right.into_iter()).map (| (first, second) | 
        // hack: convert stereo to mono
        (
          first*2f32.powi(15)
          + second*2f32.powi(15)
        ).round() as Sample).collect()
      }
    })
  }
}

// TODO: take a less specific "collection of sequences" argument type
fn merge(sequences: &Vec<Sequence>) -> Sequence {
  let mut minimum = Position::max_value();
  let mut maximum = Position::min_value();
  for sequence in sequences {
    minimum = min(minimum, sequence.start);
    maximum = max(maximum, sequence.start + sequence.samples.len() as Position);
  }
  let mut samples: Vec<Sample> = vec! [0; (maximum - minimum) as usize];
  for sequence in sequences {
    for (index, sample) in sequence.samples.iter().enumerate() {
      samples[(sequence.start - minimum) as usize + index] += *sample;
    }
  }
  Sequence {
    start: minimum,
    samples: samples,
  }
}

pub fn enforce_maximum(sequence: &mut Sequence, forced_maximum: Sample) {
  let maximum = sequence.samples.iter().fold(0, |maximum, sample| max(maximum, sample.abs()));
  if maximum <= forced_maximum {
    return;
  }
  for sample in sequence.samples.iter_mut() {
    *sample = (*sample) * forced_maximum / maximum;
  }
}

mod optimizer {

use rand::Rng;

  struct Note {
    original_frequency: f64,
    current_frequency: f64,
    neighbors: Vec<usize>,
  }

  struct Parameters {
    rounds: usize,
  }

  struct Optimizer<'lifetime> {
    notes: &'lifetime mut Vec<Note>,
    parameters: Parameters,
  }

  impl<'lifetime> Optimizer<'lifetime> {
    fn run<Generator: Rng>(&mut self, generator: &mut Generator) {
      for _ in 0..self.parameters.rounds {
        let which = generator.gen_range(0, self.notes.len());
        self.optimize_note(which);
      }
    }

    fn optimize_note(&mut self, which: usize) {
      let subject = self.notes.get(which).unwrap();
      let mut score = 0.0;

      for neighbor in subject.neighbors.iter () {

      }

    }
  }


}


// trait Interpreter


struct BasicInterpreter<Render: Renderer> {
  notes: Notes<Render>,
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

pub fn scrawl_MIDI_notes(scrawl: &str) -> Notes<MIDINote> {
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

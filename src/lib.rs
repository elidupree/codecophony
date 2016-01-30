extern crate fluidsynth;
extern crate hound;

use std::cmp::{min, max};
use std::collections::HashMap;
use std::str::FromStr;

pub type Position = i32;
pub type Sample = i32;
pub type Semitones = i32;
pub const SEMITONE_RATIO: f64 = (1.0594631f64);

#[derive (Clone)]
pub struct Sequence {
  start: Position,
  pub samples: Vec<Sample>,
}

#[derive (Clone, Copy)]
pub struct NoteBasics {
  start: f64,
  duration: f64,
}

// trait SequenceTransform<RendererType: Renderer> : Clone + Fn (&mut Sequence, &Note <RendererType>)->() {}

#[derive (Clone)]
pub struct Note<RendererType: Renderer> {
  basics: NoteBasics,
  renderer: RendererType, // sequence_transforms: Vec<Box <SequenceTransform <RendererType> >>,
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
impl <Render: Renderer> Default for Notes <Render> {
fn default ()->Self {Notes:: <Render> {data: Default::default ()}}
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
bank: i16,
preset: i16,
}
const PERCUSSION_CHANNEL:i32 = 9;
impl MIDIInstrument {
//offsets the program by one to use the same numbers as the General MIDI specification, which numbers the instruments from one rather than 0
pub fn new (program: i16)->Self {MIDIInstrument {bank: 0, preset: program -1, channel: 0}}
pub fn percussion ()->Self {MIDIInstrument {bank: 0, preset: 0, channel: PERCUSSION_CHANNEL}}
pub fn is_percussion (& self)->bool {self.channel == PERCUSSION_CHANNEL}
}

#[derive (Clone)]
pub struct MIDINote {
pitch: i16,
velocity: i16,
instrument: MIDIInstrument,
}
impl Transposable for MIDINote {
fn transpose (&mut self, amount: Semitones)->& mut Self {
self.pitch += amount as i16; self
}
}
impl Renderer for MIDINote {
fn render (& self, basics: NoteBasics, sample_rate: Position)->Sequence {
{
  let mut settings = fluidsynth::settings::Settings::new ();
  settings.setstr ("audio.file.name", "test_render.wav");
  settings.setstr ("audio.file.type", "wav");
  settings.setnum ("synth.sample-rate", sample_rate as f64);
  
  let mut synthesizer = fluidsynth::synth::Synth::new (&mut settings);
  let mut sequencer = fluidsynth::seq::Sequencer::new2 (0);
let sequencer_ID = sequencer.register_fluidsynth (&mut synthesizer);
  let mut renderer = fluidsynth::audio::FileRenderer::new (&mut synthesizer);
  
  let font_ID = synthesizer.sfload ("/usr/share/sounds/sf2/FluidR3_GM.sf2", 1).unwrap ();
  
  let send_event = | time, assign: & Fn (&mut fluidsynth::event::Event) | {
  
  let mut event = fluidsynth::event::Event::new ();
event.set_source (-1); event.set_destination (sequencer_ID);
assign (&mut event);
sequencer.send_at (&mut event, time, 1);
  };
  if !self.instrument.is_percussion () {send_event (0, & | event | event.program_select (self.instrument.channel, font_ID, self.instrument.bank, self.instrument.preset));}
  send_event (0,
& | event | event.noteon (self.instrument.channel, self.pitch, self.velocity));
if  !self.instrument.is_percussion () {send_event ((basics.duration *1000.0) as u32, & | event |
event.noteoff (self.instrument.channel, self.pitch));}

//TODO: instead of just using twice the duration, specifically continue rendering until we get all zeros
for _ in 0..(2.0*basics.duration* settings.getnum ("synth.sample-rate").unwrap () /settings.getint ("audio.period-size").unwrap () as f64) as i32 {renderer.process_block ();}
}
//the settings change above didn't work, for some reason, so the file is@" fluidsynth.wav"
let mut reader = hound::WavReader::open ("fluidsynth.wav").unwrap ();
//hack: convert stereo to mono
let mut samples = Vec::new ();
let mut iterator =reader.samples::<i32> ().map (| result | result.unwrap ());
while let Some (sample) = iterator.next () {
samples.push ((sample + iterator.next ().unwrap ())/2);
}
    Sequence {
      start: (basics.start * sample_rate as f64) as Position,
      samples: samples,
    }

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

mod optimizer {

struct Note {
original_frequency: f64,
current_frequency: f64,
neighbors: Vec<usize>,
}

struct Parameters {
rounds: usize,
}

struct Optimizer <'lifetime> {
notes: & 'lifetime mut Vec<Note>, parameters: Parameters
}

impl < 'lifetime>Optimizer < 'lifetime> {

fn run (&mut self) {
for _ in 0..self.parameters.rounds {
let TODO = 0; self.optimize_note (TODO);
}
}

fn optimize_note (&mut self, which: usize) {

}
}


}


//trait Interpreter


struct BasicInterpreter <Render: Renderer > {
notes: Notes <Render>,
now: f64,
step_size: f64,
sustained_notes: HashMap <Semitones, Note <Render>>,
latest_notes: HashMap <Semitones, Note <Render>>,
command_in_progress: Option <String>,
}



trait InterpreterCaller <Render: Renderer> {
fn create (&mut self, semitones: Semitones)->Render;
}
impl <Render: Renderer> Default for BasicInterpreter <Render> {
fn default ()->Self {BasicInterpreter:: <Render> {now: 0.0, step_size: 1.0, notes: Default ::default (), sustained_notes: Default::default (), latest_notes: Default::default (), command_in_progress: None,}}
}
impl <Render: Renderer> BasicInterpreter <Render> {


fn finish_note (& mut self, note: Note <Render>) {
let mut note = note;
note.basics.duration = self.now - note.basics.start;//note.set_end (self.now);
      self.notes.push (note);
}
fn finish_notes (&mut self) {
    let last_begin = self.latest_notes.values().fold(-900000000.0f64, |max, note: & Note <Render> | {
      if note.basics.start> max {
note.basics.start
      } else {
        max
      }
    });
    let step_end = last_begin + self.step_size;
    if step_end > self.now {
self.now = step_end
    };

    for (_, note) in self.latest_notes.clone ().iter() {
self.finish_note (note.clone ());
    }
self.latest_notes.clear ();

}

fn create_note <Caller: InterpreterCaller <Render>> (&mut self, caller: &mut Caller, semitones: Semitones)->Note <Render> {
Note::<Render> {basics: NoteBasics {start: self.now, duration: 0.0}, renderer: caller.create (semitones)}
}

fn interpret <Caller: InterpreterCaller <Render>> (&mut self, caller: &mut Caller, command: & str) {
match self.command_in_progress.clone () {

None => match Semitones::from_str (command) {
Ok (semitones) => {self.finish_notes (); let note = self.create_note (caller, semitones); self.latest_notes.insert (semitones, note);},
Err (_) => match command {
"finish" => self.finish_notes (),
_=> self.command_in_progress = Some (command.to_string ()),
}
},
Some (last_command) => {match &*last_command {
"and" => {let semitones = Semitones::from_str (command).unwrap ();
let note = self.create_note (caller, semitones); self.latest_notes.insert (semitones, note);},
"sustain" => {let semitones = Semitones::from_str (command).unwrap (); let note = self.create_note (caller, semitones); self.sustained_notes.insert (semitones, note );},
"release" => {let semitones = Semitones::from_str (command).unwrap (); let note = self.sustained_notes.remove (& semitones).unwrap (); self.finish_note (note);},
"step" => {self.step_size = f64::from_str (command).unwrap ();},
_=> panic! (),
}; self.command_in_progress = None;}
,

}}


}

struct MIDIInterpreter {

prototype: MIDINote,
velocity_adjustment: i16,
command_in_progress: Option <String>,
}

impl InterpreterCaller <MIDINote> for MIDIInterpreter {
fn create (&mut self, semitones: Semitones)->MIDINote {
let mut velocity = self.prototype.velocity;
while self.velocity_adjustment >0 {self.velocity_adjustment -= 1; velocity = (velocity*2 +128)/3;}
while self.velocity_adjustment <0 {self.velocity_adjustment += 1; velocity = (velocity*2)/3;}
MIDINote {pitch: self.prototype.pitch + semitones as i16, velocity: velocity ,..self.prototype.clone ()}
}
}

impl MIDIInterpreter {
fn interpret (&mut self, basics: &mut BasicInterpreter <MIDINote>,command: & str) {
match self.command_in_progress.clone () {
None => match command {
"strong" => self.velocity_adjustment += 1,
"quiet" => self.velocity_adjustment -= 1,
"percussion" => self.prototype.instrument = MIDIInstrument::percussion (),
parametric@"instrument" | parametric@"velocity" | parametric@"transpose" => self.command_in_progress = Some (parametric.to_string ()),
other => basics.interpret (self, other),
},
Some (last_command) => {
match &*last_command {
"instrument" => self.prototype.instrument = MIDIInstrument::new (i16::from_str (command).unwrap ()),
"velocity" => self.prototype.velocity =i16::from_str (command).unwrap (),
"transpose" => self.prototype.pitch =i16::from_str (command).unwrap (),
_=> panic! (),
};
self.command_in_progress = None;
}
}
}
}

pub fn scrawl_MIDI_notes (scrawl: & str)->Notes <MIDINote> {
let mut basics = BasicInterpreter:: <MIDINote>::default ();
let mut specifics = MIDIInterpreter {velocity_adjustment: 0, prototype: MIDINote {pitch: 0, velocity: 64, instrument: MIDIInstrument::new (88)}, command_in_progress: None};
for command in scrawl.split_whitespace () {
specifics.interpret (&mut basics, command);
}
basics.finish_notes ();
basics.notes
}

use codecophony::*;
use rand::{self, Rng, SeedableRng, ChaChaRng};
use std::collections::BTreeMap;
use siphasher::sip::SipHasher;
use std::hash::{Hash, Hasher};

pub const SAMPLE_HZ: f64 = 44100.0;
pub const CHANNELS: usize = 2;
pub type Output = f32;

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
enum Timbre {
  Percussion {instrument: i32},
  Pitched {instrument: u32, pitch: i32}
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct Note {
  start: i32,
  duration: i32,
  timbre: Timbre,
}

impl Note {
  fn to_renderable (&self, scale: f64, volume: f64)->Box<Renderable<[Output; CHANNELS]> + Send> {
    let time = self.start as f64*scale;
    let duration = self.duration as f64*scale;
    match self.timbre {
      Timbre::Percussion {instrument} => Box::new(MIDIPercussionNote::new(time, 1.0, (100.0*volume) as i32, instrument)),
      Timbre::Pitched {instrument, pitch} => {
        let velocity_penalty = max (0, pitch - 60);
        Box::new(MIDIPitchedNote::new(time, duration, pitch, (100.0*volume) as i32 - velocity_penalty, instrument))
      }
    }
  }
} 

use std::cmp::{min,max};
fn random_timbre <G: Rng> (generator: &mut G)->Timbre {
  if generator.gen() {
    let mut instrument = generator.gen_range(35, 83);
    while instrument == 58 || instrument == 71 || instrument == 72 { instrument = generator.gen_range(35, 83); }
    Timbre::Percussion {instrument}
  }
  else {
    /*let frequency: f64 = ((generator.gen::<f64>()*2f64-1f64)+(220f64).ln()).exp();
    let mut amplitude = 0.2*volume*220.0/frequency;
    if amplitude > 0.5*volume { amplitude = 0.5*volume.sqrt(); } 
    Rc::new(move |time| vec![Box::new(codecophony::SineWave { start: time, duration, frequency, amplitude})])*/
    let instrument = generator.gen_range(1, 120);
    let pitch = generator.gen_range(33, 81);
    Timbre::Pitched {instrument, pitch}
  }
}

//dff


#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct NotesSelector {
  probability: u64,
  seed: u64,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
enum PlaceChange {
  DeleteNotes (NotesSelector),
  CreateNote (Timbre),
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct Pattern {
  start: u64,
  fixed_levels: u64,
  ordering: u64,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct PlaceChangePattern {
  change: PlaceChange,
  pattern: Pattern,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, Default)]
struct PlaceData {
  changes: BTreeMap<u64, PlaceChangePattern>,
  //notes: Vec<Note>,
}



impl NotesSelector {
  fn applies_to (&self, note: & Note)->bool {
    let mut hasher = SipHasher::new();
    self.seed.hash (&mut hasher);
    note.timbre.hash (&mut hasher);
    hasher.finish() <self.probability
  }
}

impl PlaceChange {
  fn apply (&self, time: u64, notes: &mut Vec<Note>) {
    match self {
      PlaceChange::CreateNote (timbre) => {
        notes.push (Note {start: time as i32, duration: 2, timbre: timbre.clone()});
      },
      PlaceChange::DeleteNotes (selector) => {
        notes.retain (| note |!selector.applies_to (note));
      },
    }
  }
}


fn random_place_change <G: Rng> (generator: &mut G)->PlaceChange {
  if generator.gen_range(0, 8) < 6 {
    PlaceChange::CreateNote (random_timbre (generator))
  } else {
    PlaceChange::DeleteNotes (NotesSelector {
      probability: 1u64<<63,
      seed: generator.gen(),
    })
  }
}

pub fn generate_music()->Box <Renderable<[Output; CHANNELS]>> {
  let mut generator = ChaChaRng::from_seed(&[35]);
  //let notes: Vec<_> = (0..500).map (| index | Note {start: index, duration: 1, timbre: random_timbre (&mut generator)}.to_renderable (0.25, 0.6)).collect();
  
  let mut places: Vec<PlaceData> = Vec::new();
  for index in 0..512 {
    let mut ancestors = Vec::new();
    let mut max_level = 0;
    for level in 0.. {
      if index & (1<<level) != 0 {
        ancestors.push((index - (1<<level), level))
      }
      if index < (1<<level) {
        max_level = level;
        break;
      }
    }
    let mut new_data = PlaceData::default();
    for (ancestor, level) in ancestors.into_iter().rev() {
      for (_, change) in & places [ancestor].changes {
        if change.pattern.fixed_levels & (1 << level) == 0 {
          new_data.changes.insert (change.pattern.ordering, change.clone());
        }
      }
    }
    
    for _ in 0..100 {
      let fixed_levels: u64 = generator.gen();
      if index as u64 & fixed_levels != index as u64 {continue}
      let mut pattern = PlaceChangePattern {
        change: random_place_change (&mut generator),
        pattern: Pattern {start: index as u64, fixed_levels, ordering: generator.gen()},
      };
      if index & 1 != 0 && generator.gen() {
        pattern.change = PlaceChange::DeleteNotes (NotesSelector {
      probability: 1u64<<63,
      seed: generator.gen(),
    });
      }
      new_data.changes.insert (pattern.pattern.ordering, pattern);
    }
    
    places.push (new_data);
  }
  
  let mut notes = Vec::new();
  for (time, place) in places.into_iter().enumerate() {
    let mut notes_here = Vec::new();
    for (_, change) in & place.changes {
      change.change.apply (time as u64, &mut notes_here) ;
    }
    notes.extend (notes_here);
  }
  let notes: Vec<_> = notes.into_iter().map (| note| note.to_renderable(1.0/8.0, 0.6)).collect();
  Box::new (notes)
}


/*

struct MusicFragment {
  // the time this fragment starts relative to the start of its parent
  offset: f64, 
  duration: f64,
  
  // nonoverlapping children.
  // at long durations, these partition this fragment's duration;
  // at shorter durations, there may be missing children
  // when those children wouldn't have any descendant notes
  children: Vec<MusicFragment>,
  
  // generally should only contain notes that don't fit in one of the children
  notes: Vec<Note>,
}

struct FragmentGenerator {
  source: MusicFragment,
  in_progress: MusicFragment,
  current_child_generator: Option <FragmentGenerator>,
  
  //transposition: i32,
}

struct MusicGenerator {
  
}
const SHORTEST_FRAGMENT: f64 = 0.1;

impl FragmentGenerator {
  fn step (&mut self) {
    if self.in_progress.duration < SHORTEST_FRAGMENT {
      
    }
    else {
      
      let no_child;
      if let Some(child) = self.current_child_generator.as_mut() {
        child.step();
        
      }
      
      {
      
      }
    }
  }
}


impl Remix for


#[derive (Clone)]
struct Pattern {
  duration: i32,
  offset: i32,
  pattern_type: PatternType,
}

#[derive (Clone)]
enum PatternType {
  Assemblage (Vec<Pattern>),
  Notes (Vec<PatternTimbre>),
}

*/





/*


#[derive (Clone)]
struct ForwardPattern {
  duration: i32,
  max_voices: i32,
  children: [Vec<ForwardPattern>; 2],
  notes: Vec<PatternTimbre>,
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct PatternPosition {
  start: i32,
  duration: i32,
}

#[derive (Clone, Serialize, Deserialize, Debug)]
struct CustomPattern {
  serial_number: u64,
  position: PatternPosition,
  max_voices: i32,
  repetitions: i32,
  children: Vec<(PatternPosition, Vec<CustomPattern>)>,
  notes: Vec<PatternTimbre>,
}

struct Subsequence {
  notes: Vec<PatternNote>,
  occurrences: Vec<i32>,
}
struct UnfinishedSubSubsequence {
  next_note: PatternNote,
  offset: i32,
}

*/

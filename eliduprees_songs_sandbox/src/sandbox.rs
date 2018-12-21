use codecophony;
use codecophony::*;
use codecophony::phrase::{Phrase, PhraseNote};
use rand::{self, Rng, SeedableRng, ChaChaRng};
use std::iter::FromIterator;
use std::collections::{HashMap, HashSet};
use ordered_float::OrderedFloat;
use std::path::Path;
use std::str::FromStr;
use std::rc::Rc;


pub const SAMPLE_HZ: f64 = 44100.0;
pub const CHANNELS: usize = 2;
pub type Output = f32;

fn applied<F: FnMut(&mut PhraseNote)> (mut collection: Vec<PhraseNote>, mut callback: F) -> Vec<PhraseNote> {
  for note in collection.iter_mut() {callback (note);}
  collection
}

fn find_tag (collection: &Vec<PhraseNote>, item: & str)->PhraseNote {
  // I don't want to have fallback cases all over the place, but
  // make a meaningless default so that we don't crash
  collection.iter().find (| whatever | whatever.tags.contains (item)).cloned().unwrap_or (PhraseNote::new (0.0, 0.0, 100.0))
}

fn concat (mut first: Vec<PhraseNote>, second: Vec<PhraseNote>)-> Vec<PhraseNote> {
  first.extend (second.into_iter());
  first
}

pub fn current_watcher() {
  let mut percussion_table = HashMap::new();
  let project_path = Path::new("../data_02");
  percussion_table.insert(30,40);
  codecophony::project::watch_phrases (&project_path, &mut |phrases, _changed| {
    
    let mut arising = phrases ["arising"].notes.clone();
    for note in arising.iter_mut() {
      if note.tags.contains ("melody") {
        let start = note.start;
        note.dilate(0.8, start);
        if !note.tags.contains ("first") {
          note.tags.insert (String::from_str ("weakened").unwrap());
        }
      }
    }
    
    let striking = phrases ["striking"].notes.clone();
    let first = concat (
      arising.clone().into_iter().filter(| note | {!note.tags.contains ("next_phrase")}).collect(),
      applied (striking.clone(), |note| note.nudge (find_tag (&arising, "next_phrase").start))
    );
    let second = applied (first.clone(), |note| note.nudge (find_tag (&first, "next_phrase").start));
    
    let first_second = concat(first.clone(), second.clone());
    
    let mut notes: Vec<Box<Renderable<[Output; CHANNELS]> + Send>> = Vec::new();
    for note in first_second.iter() {
      let velocity = if note.tags.contains ("emphasis") {120} else if note.tags.contains ("weakened") {60} else {90};
      if note.tags.contains ("percussion") {
        let pitch = frequency_to_nearest_midi_pitch(note.frequency);
        let instrument = percussion_table.get(&pitch).cloned().unwrap_or(35);
        notes.push(Box::new(MIDIPercussionNote::new (note.start, note.end - note.start, velocity, instrument)));
      }
      else {
        let instrument = if note.tags.contains ("melody") {57} else {43};
        notes.push(Box::new(MIDIPitchedNote::new (note.start, note.end - note.start, frequency_to_nearest_midi_pitch (note.frequency), velocity, instrument)));
      }
    }
    
    codecophony::project::write_phrase (&project_path, "output", &Phrase {notes: first_second.clone()});
    codecophony::project::set_playback_data (&project_path, SAMPLE_HZ, Some(Box::new(notes)));
  });
}


pub fn current_input_playground (input: & HashMap <String, Phrase>) -> (Box<Renderable<[Output; CHANNELS]> + Send>, Vec<Phrase>) {
  /*let notes = input.get("first_test").unwrap().to_midi_pitched (| note | {
    let instrument = if note.tags.contains ("melody") {61} else {43};
    (90, instrument)
  });*/
  
  let mut notes = Vec::new();
  
  let mut arising: Vec<_> = input.get("arising").unwrap().notes.iter().map(| note | {
    let instrument = if note.tags.contains ("melody") {57} else {43};
    let velocity = if note.tags.contains ("first") {90} else {60};
    let mut duration = note.end - note.start;
    if note.tags.contains ("melody") {duration *= 0.8;};
    (note, MIDIPitchedNote::new (note.start, duration, frequency_to_nearest_midi_pitch (note.frequency), velocity, instrument))
  }).collect();
  for note in arising.iter_mut() {
    note.1.dilate(3.0/4.0, 0.0);
  }
  let striking: Vec<_> = input.get("striking").unwrap().to_midi_pitched (| note | {
    let instrument = if note.tags.contains ("melody") {57} else {43};
    let velocity = if note.tags.contains ("emphasis") {120} else if note.tags.contains ("emphasis") {60} else {90};
    (velocity, instrument)
  }).into_iter().map(|mut note| {
    note.nudge(12.0/4.0);
    note
  }).collect();
  notes.extend(arising.iter().map(|n| n.1.clone()));
  notes.extend(striking.iter().cloned());
  notes.extend(arising.iter().map(|n| n.1.clone()).map(|mut note| {
    note.nudge(29.0/4.0);
    note
  }));
  notes.extend(arising.iter().cloned().filter_map(|mut note| {
    if note.0.tags.contains ("bass") {
      note.1.nudge(29.0/4.0);
      note.1.transpose(7);
      Some(note.1)
    } else {None}
  }));
  notes.extend(striking.iter().cloned().map(|mut note| {
    note.nudge(29.0/4.0);
    note
  }));
  let phrases = vec![Phrase::from_iter (notes.iter())];
  (Box::new(notes), phrases)
}

pub fn current_playground() -> (Box<Renderable<[Output; CHANNELS]> + Send>, Vec<Phrase>) {
  
  /*let note = codecophony::SineWave {
    start:0.0, duration:1.0,
    frequency: 265.0, amplitude: 0.25,
  };*/
  
  /*let mut notes: Vec<_> = (0..100u32).map(|index| codecophony::SineWave {
    start: index as f64 * 0.3, duration:1.0,
    frequency: 220.0, amplitude: 0.1,
  }).collect();
  
  
  codecophony::interval_optimizer::optimize_notes (&mut notes,
    codecophony::interval_optimizer::OptimizeNotesParameters {max_change_ratio: 2.0, .. Default::default()},
    |(_note, frequency), neighbors| {
      let mut result = 0.0;
      for &(_, neighbor_frequency) in neighbors.iter() {
        let interval = codecophony::interval_optimizer::closest_reference_interval (frequency/neighbor_frequency);
        let error = ((interval.frequency()/frequency)-1.0).powi(2);
        let limit_score = if interval.odd_limit == 1 {
          if ((frequency/neighbor_frequency)-1.0).abs() < 0.5 {
            // unison bad!
            -13.0
          }
          else {
            // octave ok
            -5.0
          }
        }
        else {
          //(interval.odd_limit as f64).ln()
          -interval.odd_limit as f64
        };
        result += limit_score - error;
      }
      result
    }
  );
  
  for note in notes.iter_mut() {note.amplitude *= 220.0/note.frequency;}*/
  
  /*let notes: Vec<_> = (0..100u32).map(|index|
    MIDIPitchedNote::new(index as f64 * 0.3, 1.0, 1+index as i32, 90, 3)
  ).collect();
  
  let notes: Vec<_> = (0..1000u32).map(|index|
    MIDIPercussionNote::new((index as f64 + 1.0).ln(), 1.0, 90, 35)
  ).collect();*/
  
  /*let beats: f64 = 4.0;
  use std::iter;
  let beat_weights: Vec<f64> =
    iter::repeat(0.0).take(8)
    .chain(iter::repeat(2.0).take(4))
    .chain(iter::repeat(1.0).take(2))
    .chain(iter::repeat(3.0).take(2))
    .chain(iter::repeat(0.5).take(1))
    .chain(iter::repeat(1.5).take(1))
    .chain(iter::repeat(2.5).take(1))
    .chain(iter::repeat(3.5).take(1))
    .collect();
  let step_weights: Vec<(f64, f64)> =
    iter::repeat((1.0,0.0)).take(1)
    .chain(iter::repeat((2.0,0.0)).take(1))
    .chain(iter::repeat((2.0,1.0)).take(1))
    .chain(iter::repeat((4.0,0.0)).take(1))
    .chain(iter::repeat((4.0,1.0)).take(1))
    .chain(iter::repeat((4.0,2.0)).take(1))
    .chain(iter::repeat((4.0,3.0)).take(1))
    .collect();
  
  let mut generator = rand::chacha::ChaChaRng::from_seed(&[35]);
  
  let mut notes = Vec::new();
  for instrument in 35..82 {
    if instrument == 58 || instrument == 71 || instrument == 72 || instrument == 78 || instrument == 79 {continue;} and
    let &beat = generator.choose (& beat_weights).unwrap();
    let &(step, phase) = generator.choose (& step_weights).unwrap();
    let mut time = beat+beats*phase;
    while time < 60.0 {
      notes.push (
        MIDIPercussionNote::new(time/4.0, 1.0, 50, instrument)
      );
      time += step*beats;
    }
  }*/
  
  /*
  let mut generator = rand::chacha::ChaChaRng::from_seed(&[35]);
  let mut freq = 220.0;
  let timeadvance = 0.2;
  let harmonics = vec![
    3.0,5.0,7.0//,9.0,11.0,13.0
  ];
  let mut notes: Vec<_> = (0..1000u32).map(|index| {
    let factor = generator.choose(&harmonics).unwrap();
    if generator.gen() {
      freq *= factor;
    }
    else {
      freq /= factor;
    }
    while freq < 220.0/(1.0f64 + generator.gen::<f64>() * 5.0f64) { freq *= 2.0; }
    while freq > 220.0*(1.0f64 + generator.gen::<f64>() * 5.0f64) { freq /= 2.0; }
    let mut amplitude = timeadvance*0.2*220.0/freq;
    if amplitude > timeadvance*0.5 { amplitude = timeadvance*0.5; } 
    codecophony::SineWave {
      start: index as f64 * timeadvance, duration:1.0,
      frequency: freq, amplitude,
    }
  }).collect();*/
  
  
  
  /*let mut generator = rand::chacha::ChaChaRng::from_seed(&[35]);
  let levels = 5;
  let patterns = vec![
    vec![0,2],
    vec![1,3],
    vec![0,1,2,3],
    vec![0,1],
    vec![2,3],
    vec![2],
  ];
  
  let mut notes: Vec<Box<Renderable<[Output; CHANNELS]> + Send>> = Vec::new();
  for instrument in 35..82 {
    if instrument == 58 || instrument == 71 || instrument == 72 || instrument == 78 || instrument == 79 {continue;}
    
    let mut my_patterns = Vec::new();
    for _ in 0..levels {my_patterns.push (generator.choose (& patterns).unwrap());}
    
    'whoops: for time in 0u32..(1<<(2*levels)) {
      for level in 0..levels {
        if my_patterns [level].iter().find (|a| **a==(time >> (2*level)) & 3).is_none() {
          continue 'whoops;
        }
      }
      notes.push (
        Box::new(MIDIPercussionNote::new(time as f64/4.0, 1.0, 50, instrument))
      );
    }
  }
  
  let mut melody_patterns = Vec::new();
  let harmonics = vec![
    3.0,5.0,7.0,
    1.0/3.0, 1.0/5.0, 1.0/7.0,
  ];
  
  let melody_levels = levels - 1;
  for _ in 0..melody_levels {melody_patterns.push ([
    *generator.choose (& harmonics).unwrap(),
    *generator.choose (& harmonics).unwrap(),
    *generator.choose (& harmonics).unwrap(),
    1.0,
  ]);}
  for time in 0u32..(1<<(2*melody_levels)) {
    for level in 0..melody_levels {
      if (time as usize >> level) & 3 == 0 {
        melody_patterns [level] = [
          *generator.choose (& harmonics).unwrap(),
          *generator.choose (& harmonics).unwrap(),
          *generator.choose (& harmonics).unwrap(),
          1.0,
        ];
      }
    }
    for level in 0..3 {
      let mut frequency = 220.0;
      for level2 in level..melody_levels {
        frequency *= melody_patterns [level2][(time as usize >> level2) & 3];
      }
      while frequency < 100.0*(3-level) as f64/(2.0) { frequency *= 2.0; }
      while frequency > 100.0*(3-level) as f64*(2.0) { frequency /= 2.0; }
      let mut amplitude = 0.1*220.0/frequency;
      if amplitude > 0.25 { amplitude = 0.25; } 
      notes.push (
        Box::new(codecophony::SineWave { start: time as f64, duration: 1.05, frequency, amplitude})
      );
    }
  }*/
  
  
  let mut generator = rand::chacha::ChaChaRng::from_seed(&[46]);
  //let notes = assemble_pattern (create_random_pattern (1<<11, 1.0, &mut generator), 0);
  //let notes = assemble_forward_pattern (& generate_forward_pattern (&mut generator, 1<<11), 0);
  //let notes = generate_familiarity_music (&mut generator, 1<<9);
  //let notes = generate_familiarity2_music (&mut generator, 1<<9);
  let pattern = generate_custom_pattern (&mut generator, 0, 1<<11, & MusicSpecification {});
  //println!("{:?}", pattern);
  let notes = assemble_custom_pattern (& pattern);
  let notes: Vec<_> = notes.into_iter().map (| note | note.to_renderable(1.0/16.0, 0.6)).collect();
  
  
  let phrases = vec![];// vec![Phrase::from_iter (notes.iter())];
  (Box::new(notes), phrases)
}





fn weighted_random<'a, T> (generator: &mut ChaChaRng, choices: &'a [(f64, T)]) -> &'a T {
  let total: f64 = choices.iter().map (| choice | choice.0).sum();
  let mut value = generator.gen::<f64>()*total;
  for choice in choices {
    value -= choice.0;
    if value < 0.0 { return &choice.1; }
  }
  &choices.last().unwrap().1
}
fn weighted_random_with<T, F: Fn(&mut ChaChaRng) -> T> (generator: &mut ChaChaRng, choices: & [(f64, F)]) -> T {
  let func = weighted_random(generator, choices);
  (func)(generator)
}






#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
enum PatternTimbre {
  Percussion {instrument: i32},
  Pitched {instrument: u32, pitch: i32}
}

#[derive (Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct PatternNote {
  start: i32,
  duration: i32,
  timbre: PatternTimbre,
}

impl PatternNote {
  fn to_renderable (&self, scale: f64, volume: f64)->Box<Renderable<[Output; CHANNELS]> + Send> {
    let time = self.start as f64*scale;
    let duration = self.duration as f64*scale;
    match self.timbre {
      PatternTimbre::Percussion {instrument} => Box::new(MIDIPercussionNote::new(time, 1.0, (100.0*volume) as i32, instrument)),
      PatternTimbre::Pitched {instrument, pitch} => {
        let velocity_penalty = max (0, pitch - 60);
        Box::new(MIDIPitchedNote::new(time, self.duration as f64*scale, pitch, (100.0*volume) as i32 - velocity_penalty, instrument))
      }
    }
  }
}

use std::cmp::{min,max};
fn random_pattern_timbre (generator: &mut ChaChaRng)->PatternTimbre {
  if generator.gen() {
    let mut instrument = generator.gen_range(35, 83);
    while instrument == 58 || instrument == 71 || instrument == 72 { instrument = generator.gen_range(35, 83); }
    PatternTimbre::Percussion {instrument}
  }
  else {
    /*let frequency: f64 = ((generator.gen::<f64>()*2f64-1f64)+(220f64).ln()).exp();
    let mut amplitude = 0.2*volume*220.0/frequency;
    if amplitude > 0.5*volume { amplitude = 0.5*volume.sqrt(); } 
    Rc::new(move |time| vec![Box::new(codecophony::SineWave { start: time, duration, frequency, amplitude})])*/
    let instrument = generator.gen_range(1, 120);
    let pitch = generator.gen_range(33, 81);
    PatternTimbre::Pitched {instrument, pitch}
  }
}

fn random_pattern_timbre_or_silence (generator: &mut ChaChaRng)->Vec<PatternTimbre> {
  if generator.gen_range(0, 3) != 0 {
    vec![random_pattern_timbre(generator)]
  }
  else {  
    vec![]
  }
}







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

fn create_random_pattern (duration: i32, duplicates: f64, generator: &mut ChaChaRng)->Pattern {
  if duplicates < 32.0 && generator.gen_range(0, 3) == 0 {
    Pattern {
      duration,
      offset: 0,
      pattern_type: PatternType::Assemblage (vec![create_random_pattern (duration, 2.0*duplicates, generator), create_random_pattern (duration, 2.0*duplicates, generator)]),
    }
  }
  else if duration > 16 || (duration as f64/16.0 > (-0.1 - generator.gen_range(0,4i32) as f64).exp2()) {
    // long patterns must be constructed from sub-patterns
    if duplicates <= 1.0 || generator.gen_range(0, 4) != 0 { if generator.gen_range(0, 3) != 0 {
      //repeating pattern
      let child = create_random_pattern (duration/2, duplicates, generator);
      let mut second_child = child.clone();
      second_child.offset += duration/2;
      Pattern {
        duration,
        offset: 0,
        pattern_type: PatternType::Assemblage (vec![child, second_child]),
      }
    }
    else {
      // differing patterns
      let child = create_random_pattern (duration/2, duplicates, generator);
      let mut second_child = create_random_pattern (duration/2, duplicates, generator);
      second_child.offset += duration/2;
      Pattern {
        duration,
        offset: 0,
        pattern_type: PatternType::Assemblage (vec![child, second_child]),
      }
    } }
    else {
      // offset pattern
      let mut child = create_random_pattern (duration/2, if duplicates > 1.5 {duplicates / 1.5} else {1.0}, generator);
      if generator.gen() { child.offset += duration/2; }
      child
    }
  }
  else {
    // short patterns are uhhh
    Pattern {
      duration,
      offset: 0,
      pattern_type: PatternType::Notes (random_pattern_timbre_or_silence (generator)),
    }
  }
}

fn assemble_pattern (pattern: Pattern, offset: i32)->Vec<PatternNote> {
  let mut result = Vec::new();
  match pattern.pattern_type {
    PatternType::Assemblage (patterns) => {
      for other_pattern in patterns {
        result.extend (assemble_pattern (other_pattern, offset + pattern.offset));
      }
    },
    PatternType::Notes (notes) => {
      let my_offset = offset + pattern.offset;
      let my_duration = pattern.duration;
      return notes.iter().map (| timbre | PatternNote {start: my_offset, duration: my_duration, timbre: timbre.clone() }).collect();
    }
  }
  result
}








#[derive (Clone)]
struct ForwardPattern {
  duration: i32,
  max_voices: i32,
  children: [Vec<ForwardPattern>; 2],
  notes: Vec<PatternTimbre>,
}


fn modify_forward_pattern (pattern: &mut ForwardPattern, generator: &mut ChaChaRng) {
  for collection in pattern.children.iter_mut() {
    for child in collection.iter_mut() {
      if generator.gen_range(0,3)!=0i32 {
        modify_forward_pattern (child, generator);
      }
    }
    collection.retain (|_| generator.gen_range(0,116)!=0i32);
    for _ in 0..3 {
      if pattern.duration > 1 && (collection.iter().map (| child | child.max_voices).sum::<i32>() as f64) < 2.0 + (pattern.duration as f64).log2() - 4.0 {
        collection.push (generate_forward_pattern (generator, pattern.duration/2));
      }
    }
  }
  /*if generator.gen() {
    let modified_children_index = generator.gen_range (0, 2);
    let reference_children_index = (modified_children_index + 1) & 1;
  }*/
  if generator.gen_range(0,118)==0i32 { 
    reroll_note (pattern, generator); 
  }
  
  update_max_voices (pattern);
}

fn update_max_voices (pattern: &mut ForwardPattern) {
  pattern.max_voices = pattern.children.iter().map (| collection | collection.iter().map (| child | child.max_voices).sum::<i32>()).max ().unwrap() + pattern.notes.len() as i32;
}

fn reroll_note (pattern: &mut ForwardPattern, generator: &mut ChaChaRng) {
  pattern.notes = Vec::new();
  
  if pattern.duration <= 16 && generator.gen::<f64>()*2.0 < (pattern.duration as f64/16.0) { 
    pattern.notes = random_pattern_timbre_or_silence (generator);
  }
  if pattern.duration > 16 && generator.gen::<f64>()*2.0 < 16.0/pattern.duration as f64 { 
    pattern.notes = random_pattern_timbre_or_silence (generator);
  }
}

fn expand_forward_pattern (pattern: ForwardPattern, generator: &mut ChaChaRng) -> ForwardPattern {
  //let pattern_type = ModifiedRepeat::new (pattern, generator);
  let mut next = pattern.clone();
  modify_forward_pattern (&mut next, generator);
  
  let duration = pattern.duration*2;
  
  let mut result = ForwardPattern {
    duration,
    max_voices: 0,
    //pattern_type,
    children: [vec![pattern], vec![next]],
    notes: Vec::new(),
  };
  reroll_note(&mut result, generator);
  update_max_voices (&mut result);
  result
}
fn generate_smallest_forward_pattern (generator: &mut ChaChaRng) -> ForwardPattern {
  let mut result = ForwardPattern {
    duration: 1,
    max_voices: 0,
    children: [vec![], vec![]],
    notes: Vec::new(),
  };
  reroll_note(&mut result, generator);
  update_max_voices (&mut result);
  result
}

fn generate_forward_pattern (generator: &mut ChaChaRng, min_duration: i32) -> ForwardPattern {
  let mut pattern = generate_smallest_forward_pattern (generator);
  while pattern.duration < min_duration {
    pattern = expand_forward_pattern (pattern, generator);
  }
  pattern
}


fn assemble_forward_pattern (pattern: & ForwardPattern, offset: i32)->Vec<PatternNote> {
  let mut result = Vec::new();
  result.extend(pattern.notes.iter().map (| timbre | PatternNote {start: offset, duration: pattern.duration, timbre: timbre.clone() }));
  for (index, children) in pattern.children.iter().enumerate() {
    for other_pattern in children.iter() {
      result.extend (assemble_forward_pattern (other_pattern, offset + index as i32*other_pattern.duration));
    }
  }
  result
}





/*
struct PatternModification {
  seed: u32,
  modification_type: PatternModificationType 
}
enum PatternModificationType {
  ClampVoices,
  NewSubpattern,
  RerollNote,
  Repeated,
  Disparate,
}
fn apply_modification (pattern: CustomPattern, modification: PatternModification) {
  let mut generator = ChaChaRng::from_seed(&[modification.seed]);
  match modification.modification_type {
    PatternModificationType::Remove => {
      
    }
  }
}*/

struct MusicSpecification {

}

impl MusicSpecification {
  fn may_delete_whole_patterns (&self, position: &PatternPosition)->bool {
    position.duration <= 32
  }
  fn voice_limit (&self, position: &PatternPosition)->i32 {
    20
  }
  fn target_voices (&self, position: &PatternPosition)->i32 {
    let virtual_duration = if position.duration > 16 && (position.start / position.duration) & 1 == 0 {
      position.duration / 2
    } else {
      position.duration
    };
    min(10, virtual_duration / 8)
  }
  fn modify_children_the_same_way_chance (&self, position: &PatternPosition)->f64 {
    0.5
  }
}

use std::cell::Cell;
thread_local! {
  static NEXT_SERIAL_NUMBER: Cell <u64> = Cell::new (0);
}
fn new_serial_number()->u64 {
  NEXT_SERIAL_NUMBER.with (| cell | {
    let result = cell.get();
    cell.set (result + 1);
    result
  })
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


fn modify_custom_pattern (
    pattern: &mut CustomPattern,
    seed: u32,
    specification: & MusicSpecification,
    callback: &Fn(
      &mut CustomPattern,
      ChaChaRng,
      &mut FnMut (
        &mut CustomPattern,
        & Fn(
          &mut CustomPattern,
          u32,
          & MusicSpecification
        )
      )
    )
  ) {
  let mut generator = ChaChaRng::from_seed(&[seed, pattern.serial_number as u32]);
  let mut child_chooser = ChaChaRng::from_seed(&[generator.gen()]);
  
  let mut child_seed_chooser = ChaChaRng::from_seed(&[generator.gen()]);
  let modify_children_the_same_way = if generator.gen::<f64>() < specification.modify_children_the_same_way_chance (&pattern.position) {Some(child_seed_chooser.gen())} else {None};
  
  let mut get_child_seed = ||->u32 {
    if let Some(seed) = modify_children_the_same_way {
      seed
    }
    else {
      child_seed_chooser.gen()
    }
  };
  
  let mut modify_child = | child: &mut CustomPattern, callback:
        & Fn(
          &mut CustomPattern,
          u32,
          & MusicSpecification
        )
 | {
    callback(child, get_child_seed(), specification);
    update_custom_max_voices (child);
  };
  
  callback (pattern, generator, &mut modify_child);
  update_custom_max_voices (pattern);
}

fn for_all_subpatterns (pattern: &mut CustomPattern, callback: &mut FnMut(
      &mut CustomPattern)) {
  callback (pattern);
  for collection in pattern.children.iter_mut() {
    for child in collection.1.iter_mut() {
      for_all_subpatterns (child, callback);
    }
  }
}
fn for_all_subpatterns_bottomup (pattern: &mut CustomPattern, callback: &mut FnMut(
      &mut CustomPattern)) {
  for collection in pattern.children.iter_mut() {
    for child in collection.1.iter_mut() {
      for_all_subpatterns (child, callback);
    }
  }
  callback (pattern);
}

fn for_last_subpatterns (pattern: &mut CustomPattern, callback: &mut FnMut(
      &mut CustomPattern)) {
  callback (pattern);
  for collection in pattern.children.last_mut().iter_mut() {
    for child in collection.1.iter_mut() {
      for_last_subpatterns (child, callback);
    }
  }
}

fn nudge_custom_pattern (pattern: &mut CustomPattern, time: i32) {
  for_all_subpatterns (pattern, &mut |foo| {
    foo.position.start += time;
    for collection in foo.children.iter_mut() {
      collection.0.start += time;
    }
  });
}

fn tweak_custom_pattern (pattern: &mut CustomPattern, seed: u32, specification: & MusicSpecification) {
  modify_custom_pattern (pattern, seed, specification, &| pattern, mut generator, modify_child | {
    let mut child_chooser = ChaChaRng::from_seed(&[generator.gen()]);
    let mut unstable = ChaChaRng::from_seed(&[generator.gen(), pattern.position.start as u32, pattern.position.duration as u32]);
    for collection in pattern.children.iter_mut() {
      for child in collection.1.iter_mut() {
        modify_child (child, &tweak_custom_pattern);
      }
    }
    while pattern.max_voices <specification.target_voices (&pattern.position) {
      println!("{:?}", (&pattern.position, &pattern.max_voices, specification.target_voices (&pattern.position)));
      //if pattern.position.start >= 256 {panic!()}
      if pattern.notes.is_empty() {
        custom_reroll_note (pattern, &mut generator);
      }
      if pattern.children.len() >0 {
        let new_child = generate_custom_pattern (&mut unstable, pattern.children [0].0.start, pattern.children [0].0.duration, specification);
        for collection in pattern.children.iter_mut().skip(1) {
          collection.1.push (make_next_sibling (& new_child, &mut unstable, specification));
        }
        pattern.children [0].1.push(new_child);
      }
      /*for collection in pattern.children.iter_mut() {
        collection.1.push (generate_custom_pattern (&mut unstable, collection.0.start, collection.0.duration, specification)); 
      }*/
      update_custom_max_voices (pattern);
      println!("{:?} er", (&pattern.position, &pattern.max_voices, specification.target_voices (&pattern.position)));
    }
    if generator.gen_range(0,118)==0i32 { 
      //custom_reroll_note (pattern, &mut generator); 
    }
    if pattern.position.duration >= 32 && generator.gen_range(0,8)==0i32 { 
      let transposition = generator.gen_range(-7,8);
      for_all_subpatterns (pattern, &mut |pattern| {
        for note in pattern.notes.iter_mut() {
          match note {
            &mut PatternTimbre::Pitched {ref mut pitch, ..} => *pitch += transposition,
            _=>(),
          };
        }
      });
    }
  });
}

fn limit_custom_pattern_voices (pattern: &mut CustomPattern, seed: u32, specification: & MusicSpecification) {
  modify_custom_pattern (pattern, seed, specification, &| pattern, mut generator, modify_child | {
    for index in 0..pattern.children.len() {
      let limit = specification.voice_limit (&pattern.children[index].0);
      while pattern.children[index].1.iter().map (| child | child.max_voices).sum::<i32>() > limit {
        reduce_custom_pattern_voices(pattern, generator.gen(), specification);
      }
      for child in pattern.children[index].1.iter_mut() {
        modify_child (child, &limit_custom_pattern_voices);
      }
    }
  });
}

fn choose_child (children: & Vec<CustomPattern>, seed: u32)->usize {
  children.iter().enumerate().max_by_key(|&(index, child)| {
    ChaChaRng::from_seed(&[seed, child.serial_number as u32]).gen::<i32>()
  }).unwrap().0
}

fn reduce_custom_pattern_voices (pattern: &mut CustomPattern, seed: u32, specification: & MusicSpecification) {
  modify_custom_pattern (pattern, seed, specification, &| pattern, mut generator, modify_child | {
    let original = pattern.max_voices;
    while pattern.max_voices == original {
      if pattern.notes.len() > 0 && generator.gen() {
        pattern.notes.pop();
      }
      else if let Some(collection) = pattern.children.iter_mut().max_by_key (|collection|
          collection.1.iter().map (| child | child.max_voices).sum::<i32>()
        ) {
        let index = choose_child (&collection.1, generator.gen());
        if (collection.1[index].max_voices <= 1) || (collection.1.len() > 1 && specification.may_delete_whole_patterns(&collection.0)) {
          collection.1.remove (index);
        }
        else {
          modify_child (&mut collection.1[index], &reduce_custom_pattern_voices);
        }
      }
      update_custom_max_voices (pattern);
    }
  });
}

  
  /*if pattern.duration > specification.max_monolithic_size {
  
  }*/
  
fn update_custom_max_voices (pattern: &mut CustomPattern) {
  pattern.max_voices = pattern.children.iter().map (| collection | collection.1.iter().map (| child | child.max_voices).sum::<i32>()).max ().unwrap_or(0) + pattern.notes.len() as i32;
}

fn custom_reroll_note (pattern: &mut CustomPattern, generator: &mut ChaChaRng) {
  pattern.notes = Vec::new();
  
  if pattern.position.duration <= 16 && generator.gen::<f64>()*2.0 < (pattern.position.duration as f64/16.0) { 
    pattern.notes = random_pattern_timbre_or_silence (generator);
  }
  if pattern.position.duration > 16 && generator.gen::<f64>()*2.0 < 16.0/pattern.position.duration as f64 { 
    pattern.notes = random_pattern_timbre_or_silence (generator);
  }
}

fn make_next_sibling(pattern: &CustomPattern, generator: &mut ChaChaRng, specification: & MusicSpecification) -> CustomPattern {
  let mut next = pattern.clone();
  
  let mut total_durations_by_serial_number = HashMap::new();
  for_all_subpatterns (&mut next, &mut | descendent |
    total_durations_by_serial_number.entry (descendent.serial_number)
      .or_insert ((descendent.position.duration, 0))
      .1 += descendent.position.duration
  );
  let removed_serial_numbers: HashSet<_> = total_durations_by_serial_number.into_iter()
    .filter(|&(number, (duration, total))| {
      println!("{:?}", (number, (duration, total, pattern.position.duration)));
      total == pattern.position.duration && duration*8 <= total && generator.gen()
    })
    .map(|(number, _)| number)
    .collect();
  
  for_all_subpatterns_bottomup (&mut next, &mut | descendent | {
    // hack: reroll IDs when children are deleted so we don't keep deleting supersets of the same pattern
    if descendent.children.iter().any(|collection| collection.1.iter().any(| child | removed_serial_numbers.contains (&child.serial_number))) {
      descendent.serial_number = ChaChaRng::from_seed(&[descendent.serial_number as u32]).gen();
    }
    for collection in descendent.children.iter_mut() {
      collection.1.retain (| child | !removed_serial_numbers.contains (&child.serial_number));
    }
    update_custom_max_voices (descendent);
  });
  
  //for_all_subpatterns (&mut next, &mut | descendent | assert!(descendent.max_voices >= specification.target_voices (& descendent.position)));
  nudge_custom_pattern (&mut next, pattern.position.duration);
  for_all_subpatterns (&mut next, &mut | descendent | descendent.repetitions *= 2);
  //for_all_subpatterns (&mut next, &mut | descendent | assert!(descendent.max_voices >= specification.target_voices (& descendent.position)));
  println!("begin {:?}", (&next.position, &next.max_voices, specification.target_voices (&next.position)));
  // TODO remove_repetitive_voices
  generator.gen::<u32>();//limit_custom_pattern_voices (&mut next, generator.gen(), specification);
  tweak_custom_pattern (&mut next, generator.gen(), specification);
  
  //let next_2 = generate_custom_pattern (generator, next.position.start, next.position.duration, specification);
  /*for collection in next.children.iter_mut() {
    collection.1.push (generate_custom_pattern (generator, collection.0.start, collection.0.duration, specification)); 
  }
  update_custom_max_voices (&mut next);*/
  
  next
}

fn expand_custom_pattern (pattern: CustomPattern, generator: &mut ChaChaRng, specification: & MusicSpecification) -> CustomPattern {
  let mut next = make_next_sibling(&pattern, generator, specification);
  
  let duration = pattern.position.duration*2;
  
  //for_all_subpatterns (&mut next, &mut | descendent | assert!(descendent.max_voices >= specification.target_voices (& descendent.position)));
  
  let mut result = CustomPattern {
    serial_number: ChaChaRng::from_seed(&[pattern.serial_number as u32]).gen(),
    position: PatternPosition {start: pattern.position.start, duration},
    max_voices: 0,
    repetitions: 1,
    //pattern_type,
    children: vec![(pattern.position.clone(), vec![pattern]), (next.position.clone(), vec![next])],
    notes: Vec::new(),
  };
  custom_reroll_note(&mut result, generator);
  update_custom_max_voices (&mut result);
  result
}
fn generate_smallest_custom_pattern (start: i32, generator: &mut ChaChaRng, specification: & MusicSpecification) -> CustomPattern {
  let mut result = CustomPattern {
    serial_number: new_serial_number(),
    position: PatternPosition {start, duration: 1},
    max_voices: 0,
    repetitions: 1,
    children: vec![],
    notes: Vec::new(),
  };
  custom_reroll_note(&mut result, generator);
  update_custom_max_voices (&mut result);
  result
}

fn generate_custom_pattern (generator: &mut ChaChaRng, start: i32, min_duration: i32, specification: & MusicSpecification) -> CustomPattern {
  let mut pattern = generate_smallest_custom_pattern (start, generator, specification);
  while pattern.position.duration < min_duration {
    pattern = expand_custom_pattern (pattern, generator, specification);
  }
  pattern
}


fn assemble_custom_pattern (pattern: & CustomPattern)->Vec<PatternNote> {
  let mut result = Vec::new();
  result.extend(pattern.notes.iter().map (| timbre | PatternNote {start: pattern.position.start, duration: pattern.position.duration, timbre: timbre.clone() }));
  for (index, children) in pattern.children.iter().enumerate() {
    for other_pattern in children.1.iter() {
      result.extend (assemble_custom_pattern (other_pattern));
    }
  }
  result
}







fn familiarity (new_timbre: &PatternTimbre, time: i32, music: &Vec<PatternNote>)-> f64 {
  let music_map: HashSet<PatternNote> = music.iter().cloned().collect();
  let analogues= music.iter().filter (| note | note.timbre == *new_timbre && note.start < time);
  let analogue_scores = analogues.map(| analogue | {
    let nudged_priors: HashSet<PatternNote> = music.iter()
      .filter (| note | note.start < analogue.start)
      .map(| note | PatternNote {start: note.start + time - analogue.start, .. note.clone()})
      .collect();
    let differences = music_map.symmetric_difference (&nudged_priors);
    let distance_to_last_difference = differences.map(|note| time - note.start).min().unwrap();
    OrderedFloat(distance_to_last_difference as f64)
  });
  analogue_scores.max().unwrap_or(OrderedFloat(0.0)).0
}

fn generate_familiarity_music (generator: &mut ChaChaRng, duration: i32)->Vec<PatternNote> {
  let mut music = Vec::new();
  let mut notes = vec![random_pattern_timbre(generator)];
  for time in 0..duration {
    let choice = //generator.choose (&notes).unwrap().clone();
      notes.iter().max_by_key(|timbre| {
        let fam = familiarity (timbre, time, &music);
        let ideal = (music.len() as f64)/32.0;//2.0;//(music.len() as f64).sqrt();
        let ideal = ((32 - ((time & 63) - 32).abs()) as f64 / 3.0);
        println!("{:?}", (time, ideal));
        OrderedFloat (-((fam - ideal).abs()) + (generator.gen::<f64>()/100.0))
      }).unwrap().clone();
    
    music.push (PatternNote {start:time, duration:1, timbre:choice.clone()});
    if choice == *notes.last().unwrap() {
      let mut new_option = random_pattern_timbre (generator);
      while notes.iter().find (| existing | **existing == new_option).is_some() {
        new_option = random_pattern_timbre (generator);
      }
      notes.push (new_option);
    }
  }
  music
}









/*fn duration (notes: & Vec<PatternNote>)->i32 {
  notes.last().unwrap().start - notes.first().unwrap().start
}*/

struct Subsequence {
  notes: Vec<PatternNote>,
  occurrences: Vec<i32>,
}
struct UnfinishedSubSubsequence {
  next_note: PatternNote,
  offset: i32,
}

fn maximal_repeating_subsequences (music: & Vec<PatternNote>)->Vec<Subsequence> {
  if music.is_empty() {return Vec::new()}
  let music_map: HashSet<PatternNote> = music.iter().cloned().collect();
  let mut result = Vec::new();
  for offset in 1..music.last().unwrap().start {
    let mut subsequence_notes = Vec::new();
    let mut first = None;
    for note in music.iter() {
      if music_map.contains (& PatternNote { start: note.start + offset, .. note.clone()}) {
        if first == None {first = Some(note.start);}
        subsequence_notes.push(PatternNote { start: note.start - first.unwrap(), .. note.clone()});
      }
    }
    if !subsequence_notes.is_empty() {
      result.push (Subsequence {notes: subsequence_notes, occurrences: vec![first.unwrap(), first.unwrap() + offset]});
    }
  }
  result
}

fn repetitiveness (music: & Vec<PatternNote>)-> f64 {
  let music_map: HashSet<PatternNote> = music.iter().cloned().collect();
  let mut max_overlap = 0;
  for offset in 1..music.last().unwrap().start {
    let mut overlap = 0;
    for note in music.iter() {
      if music_map.contains (& PatternNote { start: note.start + offset, .. note.clone()}) {
        overlap += 1;
      }
    }
    if overlap > max_overlap {max_overlap = overlap;}
  }
  let fraction_overlap = max_overlap as f64 / music.len() as f64;
  fraction_overlap / (1.0 - fraction_overlap)
}

fn unfinished_subset (pattern: & Subsequence, music: & Vec<PatternNote>)->Option <UnfinishedSubSubsequence> {
  let current_last = music.last().unwrap().start;
  let music_map: HashSet<PatternNote> = music.iter().cloned().collect();
  'outer: for offset in (current_last - pattern.notes.last().unwrap().start + 1)..(current_last+1) {
    for note in pattern.notes.iter() {
      let new_start = note.start + offset;
      let offset_note = PatternNote { start: new_start, .. note.clone()};
      if new_start > current_last {
        return Some(UnfinishedSubSubsequence {
          next_note: offset_note, offset
        })
      }
      if !music_map.contains (&offset_note) {
        continue 'outer;
      }
    }
  }
  None
}

fn add_familiarity2_note (generator: &mut ChaChaRng, music: &mut Vec<PatternNote>, end: i32) {
  if music.is_empty() {
    music.push (PatternNote {start:0, duration:1, timbre:random_pattern_timbre(generator)});
  }

  let patterns = maximal_repeating_subsequences (music);
  let mut scores: HashMap <PatternNote, f64> = HashMap::new();
  
  let current_last = music.last().unwrap().start;
  
  for pattern in patterns.iter() {
    if let Some(subset) = unfinished_subset (pattern, music) {
      let entry = scores.entry (subset.next_note.clone()).or_insert (0.0);
      let importance = pattern.notes.len() as f64;/*iter().map (| note | {
        let offset_time = notes.start + subset.offset;
        if offset_time > current_last {0.0} else {
        
        }
      }).sum();*/
      if pattern.notes.last().unwrap().start + subset.offset <= end && repetitiveness(&pattern.notes) < 4.0 {
        *entry += importance;
      }
      else {
        *entry -= importance;
      }
    }
    else if let Some(&offset) = pattern.occurrences.first() {
      let pattern_stride = pattern.occurrences.last().unwrap() - offset;
      let pattern_end = pattern.notes.last().unwrap().start + offset;
      let observed_next = music.iter().filter (| note | note.start > pattern_end).min_by_key(|note| note.start).unwrap();
      let importance = pattern.notes.len() as f64 / 100.0;
      let next_note = PatternNote { start: observed_next.start + pattern_stride, .. observed_next.clone()};
      if next_note.start >= current_last {
        let entry = scores.entry (next_note.clone()).or_insert (0.0);
        *entry += importance;
      }
    }
  }
  
  let next_note = scores.into_iter().filter (| pair | pair.0.start < current_last + 8 && pair.1 > 0.0).min_by_key(| pair | pair.0.start).map(|pair| pair.0).unwrap_or_else(|| {
    let time = match music.last() {
      None => 0,
      Some(note) => note.start + note.duration,
    };
    PatternNote {start:time, duration:1, timbre:random_pattern_timbre(generator)}
  });
  music.push(next_note);
}

fn generate_familiarity2_music (generator: &mut ChaChaRng, min_duration: i32)->Vec<PatternNote> {
  let mut music = Vec::new();
  while music.last().map_or(true, |note: & PatternNote | note.start < min_duration) {
    add_familiarity2_note (generator, &mut music, min_duration);
  }
  music
}

/*

struct FamiliarityMusic {
  existing_patterns: Vec<Vec<Rc<FamiliarityPattern>>>,
  components: Vec<Rc<FamiliarityPattern>>,
}


fn similarity (subject_pattern: ForwardPattern, reference_pattern: ForwardPattern, offset: i32) {

}

fn generate_familiarity_pattern (generator: &mut ChaChaRng, min_duration: i32) -> ForwardPattern {
  let mut pattern = generate_smallest_forward_pattern (generator);
  while pattern.duration < min_duration {
    pattern = expand_forward_pattern (pattern, generator);
  }
  pattern
}
*/

/*
struct FamiliarityPattern {
  plays: Cell <usize>,
  level: u32,
  max_voices: i32,
  children: Vec<Vec<Rc<FamiliarityPattern>>>,
  original: Option <Rc<FamiliarityPattern>>,
  similarity_to_original: f64,
  notes: Rc<Fn(f64)->Vec<Box<Renderable<[Output; CHANNELS]> + Send>>>,
}

struct FamiliarityMusic {
  existing_patterns_by_level: Vec<Vec<Rc<FamiliarityPattern>>>,
  components: Vec<Rc<FamiliarityPattern>>,
}

impl FamiliarityPattern {
  fn virtual_plays (&self)->f64 {
    (self.plays.get()+1) as f64 + self.original.map_or (0.0, |original| self.similarity_to_original*original.virtual_plays())
  }
  fn my_level_novelty (&self)->f64 {
    1.0/self.virtual_plays() 
  }
  fn total_novelty (&self)->f64 {
    self.my_level_novelty () + self.children.iter().flat_map (| children | children.iter()).map (| child | child.total_novelty ()).sum()
  }
  fn component_novelty (&self, path: &[usize])->f64 {
    let my_score = self.my_level_novelty ();
    match path.first() {
      None => self.total_novelty (),
      Some(index) => {
        self.my_level_novelty () / ((1<<path.len()) as f64)
        + self.children [index].iter().map (| child | child.component_novelty(path[1..])).sum()
      }
    }
  }
  
  fn mutated (pattern: Rc<Self>, music: & FamiliarityMusic)->FamiliarityPattern {
    FamiliarityPattern {
      level: pattern.level,
      original: Some(pattern.clone()),
    }
  }
}
*/



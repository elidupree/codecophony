use super::*;

use rand::{Rng, SeedableRng};


#[derive (Copy, Clone, Debug)]
pub struct OddLimitTableEntry {
  pub numerator: i32,
  pub denominator: i32,
  pub odd_limit: i32,
}

impl OddLimitTableEntry {
  pub fn frequency (&self)->f64 {self.numerator as f64/self.denominator as f64}
}

// Generate a table for looking up the closest pure interval.
// Technically, this could include intervals was arbitrarily high odd limit.
// But we'll leave out any intervals that are less than JND away from another interval with a lower odd limit.

lazy_static! {
  static ref CLOSEST_ODD_LIMIT_TABLE: Vec<OddLimitTableEntry> = {
    let mut result = Vec::new();
    result.push (OddLimitTableEntry {
      numerator: 1,
      denominator: 1,
      odd_limit: 1,
    });
    result.push (OddLimitTableEntry {
      numerator: 2,
      denominator: 1,
      odd_limit: 1,
    });
    
    for limit in (3..900).step_by(2) {
      for other in (1..limit).step_by(2) {
        fn consider (table: &mut Vec<OddLimitTableEntry>, limit: i32, mut numerator: i32, mut denominator: i32) {
          while numerator > denominator*2 {
            denominator *= 2;
          }
          while denominator > numerator {
            numerator *= 2;
          }
          let next_index = match table.binary_search_by(|other| {
            (other.numerator*denominator).cmp (&(numerator*other.denominator))
          }) {
            Ok(_) => return, // same interval! Already considered!
            Err(index) => index,
          };
          
          let entry = OddLimitTableEntry {
            numerator: numerator,
            denominator: denominator,
            odd_limit: limit,
          };
          
          //printlnerr!("{:?}", (limit, numerator, denominator, next_index, &table));
          if table [next_index].odd_limit < limit && table [next_index].frequency()/entry.frequency() < JUST_NOTICEABLE_FREQUENCY_RATIO {
            //printlnerr!("Discarded {:?} for being too close to {:?}", entry, table [next_index]);
            return;
          }
          if table [next_index-1].odd_limit < limit && entry.frequency()/table [next_index-1].frequency() < JUST_NOTICEABLE_FREQUENCY_RATIO {
            //printlnerr!("Discarded {:?} for being too close to {:?}", entry, table [next_index-1]);
            return;
          }          
          table.insert(next_index, entry);
        }
        
        consider (&mut result, limit, limit, other);
        consider (&mut result, limit, other, limit);
      }
    }
    
    for interval in result.iter() {printlnerr!("{:?}", interval)}
    result
  };
}

pub fn closest_reference_interval (mut frequency: f64)->OddLimitTableEntry {
  while frequency < 1.0 { frequency *= 2.0; }
  while frequency > 2.0 { frequency *= 0.5; }
  let next_index = match CLOSEST_ODD_LIMIT_TABLE.binary_search_by_key (&OrderedFloat(frequency), |other| OrderedFloat(other.frequency())) {
    Ok(x) => return CLOSEST_ODD_LIMIT_TABLE[x],
    Err(x) => x,
  };
  let next = CLOSEST_ODD_LIMIT_TABLE[next_index];
  let previous = CLOSEST_ODD_LIMIT_TABLE[next_index-1];
  
  let ratio_to_next = next.frequency()/frequency;
  let ratio_from_previous = frequency/previous.frequency();
  
  if ratio_from_previous <= ratio_to_next {
    previous
  }
  else {
    next
  }
}

pub struct OptimizeNotesParameters {
  pub iterations: usize,
  pub max_change_ratio: f64,
  pub seed: u32,
}

impl Default for OptimizeNotesParameters {
  fn default()->Self {
    OptimizeNotesParameters {
      iterations: 1000,
      max_change_ratio: 1.015,
      seed: 0,
    }
  }
}

pub fn optimize_notes <N: Windowed + Pitched + PitchShiftable + ::std::fmt::Debug, Q: Fn((&N, f64), &[(&N, f64)])->f64> (notes: &mut [N], parameters: OptimizeNotesParameters, quality: Q) {
  struct NoteMetadata {
    original_frequency: f64,
    proposed_frequency: f64,
    neighbors: Vec<usize>,
  }
  let mut notes_metadata: Vec<_> = notes.iter().map (| note | NoteMetadata {
    original_frequency: note.frequency(),
    proposed_frequency: note.frequency(),
    neighbors: Vec::new(),
  }).collect();
  
  for (index, note) in notes.iter().enumerate() {
    for (other_index, other) in notes.iter().enumerate() {
      if note.start() < other.end() && other.start() < note.end() {
        notes_metadata[index].neighbors.push(other_index);
      }
    }
  }
  
  let mut generator = rand::chacha::ChaChaRng::from_seed(&[parameters.seed]);
  let ln_max_change_ratio = parameters.max_change_ratio.ln();
  let starting_adjustment_log = ln_max_change_ratio*2.0;
  let final_adjustment_log = 1.001f64.ln();
  for iteration in 0..parameters.iterations {
    let progress = iteration as f64/(parameters.iterations-1) as f64;
    let adjustment_log = starting_adjustment_log*(1.0 - progress) + final_adjustment_log*progress;
    for index in 0..notes_metadata.len() {
      let mut new_proposal;
      loop {
        new_proposal = notes_metadata [index].proposed_frequency * (adjustment_log*(generator.gen_range(-1.0, 1.0))).exp();
        let ln_new_proposed_ratio = (new_proposal/notes_metadata [index].original_frequency).ln().abs();
        if ln_new_proposed_ratio <= ln_max_change_ratio { break }
      }
      
      let neighbors: Vec<_> = notes_metadata [index].neighbors.iter().map (| &neighbor_index |
          (&notes [neighbor_index], notes_metadata [neighbor_index].proposed_frequency)
        ).collect();
      let old_quality = quality (
        (&notes [index], notes_metadata [index].proposed_frequency),
        &neighbors
      );
      let new_quality = quality (
        (&notes [index], new_proposal),
        &neighbors
      );
      if new_quality >= old_quality {
        notes_metadata [index].proposed_frequency = new_proposal;
      }
      //printlnerr!("{:?}", (old_quality, new_quality, neighbors));
    }
  }
  
  for (index, note) in notes.iter_mut().enumerate() {
    let metadata = &notes_metadata [index];
    note.pitch_shift (metadata.proposed_frequency/metadata.original_frequency);
  }
}

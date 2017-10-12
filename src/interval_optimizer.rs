use super::*;




#[derive (Copy, Clone, Debug)]
pub struct OddLimitTableEntry {
  numerator: i32,
  denominator: i32,
  odd_limit: i32,
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
    
    for limit in (3..90).step_by(2) {
      for other in (1..limit).step_by(2) {
        fn consider (table: &mut Vec<OddLimitTableEntry>, limit: i32, mut numerator: i32, mut denominator: i32) {
          while numerator > denominator*2 {
            denominator *= 2;
          }
          while denominator > numerator {
            numerator *= 2;
          }
          let next_index = match table.binary_search_by(|other| {
            (numerator*other.denominator).cmp (&(other.numerator*denominator))
          }) {
            Ok(_) => return, // same interval! Already considered!
            Err(index) => index,
          };
          
          let entry = OddLimitTableEntry {
            numerator: numerator,
            denominator: denominator,
            odd_limit: limit,
          };
          
          if table [next_index].frequency()/entry.frequency() < JUST_NOTICEABLE_FREQUENCY_RATIO {
            printlnerr!("Discarded {:?} for being too close to {:?}", entry, table [next_index]);
            return;
          }
          if entry.frequency()/table [next_index-1].frequency() < JUST_NOTICEABLE_FREQUENCY_RATIO {
            printlnerr!("Discarded {:?} for being too close to {:?}", entry, table [next_index-1]);
            return;
          }          
          table.insert(next_index, entry);
        }
        
        consider (&mut result, limit, limit, other);
        consider (&mut result, limit, other, limit);
      }
    }
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

struct OptimizeNotesParameters {
  iterations: usize,
}

fn optimize_notes <N: Pitched + PitchShiftable> (notes: &mut [N], parameters: OptimizeNotesParameters) {
  struct NoteMetadata {
    original_frequency: f64,
    proposed_frequency: f64,
  }
  let mut notes_metadata: Vec<_> = notes.iter().map (| note | NoteMetadata {
    original_frequency: note.frequency(),
    proposed_frequency: note.frequency(),
    
  }).collect();
  for iteration in 0..parameters.iterations {
    let adjustment_scale = 0.0;
    for index in 0..notes_metadata.len() {
      
    }
  }
}

use super::*;


use std::collections::HashSet;

#[derive (Clone, Serialize, Deserialize, Debug)]
pub struct PhraseNote {
  pub start: NoteTime,
  pub end: NoteTime,
  pub frequency: f64,
  pub tags: HashSet<String>,
}

impl PhraseNote {
  pub fn new (start: NoteTime, end: NoteTime, frequency: f64)->PhraseNote {
    PhraseNote {start, end, frequency, tags: HashSet::new()}
  }
}


impl Nudgable for PhraseNote {
  fn nudge(&mut self, distance: NoteTime) {
    self.start += distance;
    self.end += distance;
  }
}

impl Dilatable for PhraseNote {
  fn dilate(&mut self, amount: f64, origin: f64) {
    self.start = origin + (self.start-origin)*amount;
    self.end   = origin + (self.end  -origin)*amount;
  }
}

impl Pitched for PhraseNote {
  fn frequency(&self)->f64 {self.frequency}
}

impl PitchShiftable for PhraseNote {
  fn pitch_shift(&mut self, frequency_ratio: f64) {
    self.frequency *= frequency_ratio;
  }
}


#[derive (Clone, Serialize, Deserialize, Debug)]
pub struct Phrase {
  pub notes: Vec<PhraseNote>,
}


impl Phrase {
  pub fn to_midi_pitched <F: FnMut (&PhraseNote)->(i32, u32)> (&self, mut velocity_and_instrument_picker: F)->Vec<MIDIPitchedNote> {
    self.notes.iter().map(| note | {
      let (velocity, instrument) = velocity_and_instrument_picker (&note);
      MIDIPitchedNote::new (note.start, note.end - note.start, frequency_to_nearest_midi_pitch (note.frequency), velocity, instrument)
    }).collect()
  }
  pub fn to_midi_percussion <F: FnMut (&PhraseNote)->(i32, i32)> (&self, mut velocity_and_instrument_picker: F)->Vec<MIDIPercussionNote> {
    self.notes.iter().map(| note | {
      let (velocity, instrument) = velocity_and_instrument_picker (&note);
      MIDIPercussionNote::new (note.start, note.end - note.start, velocity, instrument)
    }).collect()
  }
}


pub trait ToPhraseNote {
  fn to_phrase_note (&self)->PhraseNote;
}

impl ToPhraseNote for SineWave {
  fn to_phrase_note (&self)->PhraseNote {
    let mut tags = HashSet::new();
    tags.insert (String::from_str ("sine").unwrap());
    PhraseNote {
      start: self.start,
      end: self.start + self.duration,
      frequency: self.frequency,
      tags,
    }
  }
}


impl ToPhraseNote for MIDIPitchedNote {
  fn to_phrase_note (&self)->PhraseNote {
    let mut tags = HashSet::new();
    tags.insert (String::from_str ("pitched").unwrap());
    // TODO: push instrument name
    PhraseNote {
      start: self.start,
      end: self.start + self.raw.duration.into_inner(),
      frequency: self.frequency(),
      tags,
    }
  }
}

impl ToPhraseNote for MIDIPercussionNote {
  fn to_phrase_note (&self)->PhraseNote {
    let mut tags = HashSet::new();
    tags.insert (String::from_str ("percussion").unwrap());
    // TODO: push instrument name
    PhraseNote {
      start: self.start,
      end: self.start + self.raw.duration.into_inner(),
      
      // note: this is technically the exact same formula as pitched instruments,
      // but it means something different. It's kind of a hack to display percussion this way,
      // but the hack is fairly nice to look at, and other MIDI users will be familiar with it.
      frequency: midi_pitch_to_frequency(self.raw.pitch),
      tags,
    }
  }
}


impl <'a, A: ToPhraseNote> FromIterator<&'a A> for Phrase {
  fn from_iter<T: IntoIterator<Item=&'a A>> (iter: T)->Phrase {
    Phrase {
      notes: iter.into_iter().map (| item | item.to_phrase_note()).collect()
    }
  }
}

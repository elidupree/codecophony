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
  fn new (start: NoteTime, end: NoteTime, frequency: f64)->PhraseNote {
    PhraseNote {start, end, frequency, tags: HashSet::new()}
  }
}

#[derive (Clone, Serialize, Deserialize, Debug)]
pub struct Phrase {
  
  pub notes: Vec<PhraseNote>,
}


pub trait ToPhraseNote {
  fn to_phrase_note (&self)->PhraseNote;
}

impl ToPhraseNote for MIDINote<MIDIPitched> {
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

impl ToPhraseNote for MIDINote<MIDIPercussion> {
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
      frequency: 440.0*SEMITONE_RATIO.powi(69+self.raw.pitch),
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

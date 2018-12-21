extern crate codecophony;
extern crate hound;
extern crate portaudio;
extern crate dsp;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use codecophony::*;
use codecophony::phrase::Phrase;
use codecophony::rendering_gui::GuiPhrase;
use dsp::sample::ToFrameSliceMut;
use dsp::Frame;
use rand::{Rng, SeedableRng};
use std::str::FromStr;
use std::sync::Mutex;
use std::collections::HashMap;

mod sandbox;

use sandbox::{SAMPLE_HZ, CHANNELS, Output};

struct Globals {
  gui: codecophony::rendering_gui::RenderingGui,
  editable_phrases: HashMap <String, Phrase>,
  needs_update: bool,
}

lazy_static! {
  static ref GUI: Mutex<Option<Globals>> = Mutex::new(None);
}

pub fn push_gui_input (json_string: String) {
  let mut guard = GUI.lock().unwrap();
  let input = serde_json::from_str (& json_string).unwrap();
  if let Some(globals) = guard.as_mut() {
    globals.gui.apply_gui_input (&input);
    match input {
      codecophony::rendering_gui::GuiInput::EditPhrase (index, phrase) => {
        globals.editable_phrases.insert (index, phrase);
        globals.needs_update = true
      }
      _=>(),
    }
  }
}

pub fn poll_updates ()->String {
  let mut guard = GUI.lock().unwrap();
  let generate_editable = guard.is_none();
  if guard.is_none() {
    let gui = codecophony::rendering_gui::RenderingGui::new(SAMPLE_HZ);
    let mut editable_phrases = HashMap::new();
        
    editable_phrases.insert (String::from_str ("first_test").unwrap(), Phrase {notes: Vec::new()});
    editable_phrases.insert (String::from_str ("arising").unwrap(), Phrase {notes: Vec::new()});
    editable_phrases.insert (String::from_str ("striking").unwrap(), Phrase {notes: Vec::new()});
    
    *guard = Some(Globals {gui, editable_phrases: editable_phrases, needs_update: true});
  }
  let globals = guard.as_mut().unwrap();
  let mut updates = globals.gui.gui_updates();
  if globals.needs_update {
    let (notes, phrases) = sandbox::current_input_playground(& globals.editable_phrases);
    globals.gui.set_playback_data (Some(notes));
    
    if generate_editable {
      updates.push (codecophony::rendering_gui::GuiUpdate::ReplacePhrase (
        String::from_str ("first_test").unwrap(),
        GuiPhrase{
          data: Phrase {notes: Vec::new()},
          timed_with_playback: false,
          editable: true,
        }
      ));
      updates.push (codecophony::rendering_gui::GuiUpdate::ReplacePhrase (
        String::from_str ("arising").unwrap(),
        GuiPhrase{
          data: Phrase {notes: Vec::new()},
          timed_with_playback: false,
          editable: true,
        }
      ));
      updates.push (codecophony::rendering_gui::GuiUpdate::ReplacePhrase (
        String::from_str ("striking").unwrap(),
        GuiPhrase{
          data: Phrase {notes: Vec::new()},
          timed_with_playback: false,
          editable: true,
        }
      ));
    }
    
    for (index, phrase) in phrases.into_iter().enumerate() {
      updates.push (codecophony::rendering_gui::GuiUpdate::ReplacePhrase (
        format!("rendered {:?}", index),
        GuiPhrase{
          data: phrase,
          timed_with_playback: true,
          editable: false,
        }
      ));
    }

    globals.needs_update = false;
  }
  
  serde_json::to_string (& updates).unwrap()
}

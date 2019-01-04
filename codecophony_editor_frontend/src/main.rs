#![feature(nll)]
#![recursion_limit="256"]

extern crate codecophony_editor_shared as shared;
#[macro_use] extern crate stdweb;
extern crate serde_json;
extern crate serde;
extern crate nalgebra;
extern crate rand;
#[macro_use] extern crate maplit;
#[macro_use] extern crate derivative;

use serde::Serialize;
use stdweb::web::event::{MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use stdweb::web::{self, IEventTarget, HtmlElement};
use stdweb::unstable::TryInto;
use stdweb::traits::*;

use rand::prelude::*;


use shared::{MessageToBackend,MessageToFrontend,PlaybackScript,Note};

pub mod misc;
use misc::{SerialNumber};
pub mod data;
pub mod draw;
pub mod edited_note;

use data::{State, DragState, DragType, with_state_mut};
use edited_note::EditedNote;
use misc::Vector;


fn send_to_backend<T: Serialize> (send: &T) {
  let s = serde_json::to_string(send).unwrap();
  println!("Sending: {}", s);
  js!{
    backend.stdin.write(@{s});
    backend.stdin.write("\n");
  }
}

fn load_json (input: String) {
  if let Ok (notes) = serde_json::from_str::<Vec<Note>> (& input) {
    with_state_mut (| state | {
      state.notes = notes.into_iter().map (| note | EditedNote::new (note)).collect();
      state.notes_changed(false);
    });
  }
}


  
impl State {
  pub fn notes_changed (&self, save: bool) {
    self.update_elements();
    send_to_backend(&MessageToBackend::ReplacePlaybackScript(PlaybackScript {
      notes: self.notes.iter().map(|a|a.note.clone()).collect(),
      end: None,
      loop_back_to: None,
    }));
    send_to_backend(&MessageToBackend::RestartPlaybackAt (Some(0.0)));
    if save {js!{
      writeFileAtomic (window.autosave_path, @{self.serialized_notes()}, function(err) {
        if (err) throw err;
      });
    }}
  }
  pub fn serialized_notes (&self)->String {
    serde_json::to_string_pretty (& self.notes.iter().map (| note | &note.note).collect::<Vec<_>>()).unwrap()
  }
}




fn mouse_position<E: IMouseEvent> (event: &E)-> Vector {
  Vector::new (event.client_x() as f64, event.client_y() as f64)
}

fn mouse_down (event: MouseDownEvent) {
  let position = mouse_position (& event);
  let target: HtmlElement = event.target().unwrap().try_into().unwrap();
  let note_id: Option<SerialNumber> = js! {
    let closest = $(@{&target}).closest ("[data-noteid]");
    //console.log(closest.attr("data-noteid"), closest);
    return parseInt(closest.attr("data-noteid"));
  }.try_into().ok().map (| number: u32 | SerialNumber (number as u64));
  let handle_type = target.get_attribute ("data-handletype").unwrap_or_else(Default::default);
  //eprintln!(" mousedown {:?}", note_id);
  with_state_mut (| state | {
    state.mouse.position = position;
    state.mouse.drag = Some (DragState {
      start_position: position,
      start_note: note_id,
      start_handle_type: handle_type,
      ever_moved_much: false,
    });
  });
}
fn mouse_move (event: MouseMoveEvent) {
  let position = mouse_position (& event);
  with_state_mut (| state | {
    state.mouse.position = position;
    state.mouse.shift_key = event.shift_key();
    state.mouse.control_key = event.ctrl_key();
    if let Some(drag) = state.mouse.drag.as_mut() {
      if (position - drag.start_position).norm() > 5.0 {
        drag.ever_moved_much = true;
      }
    }
    state.update_elements();
  });
}
fn mouse_up (event: MouseUpEvent) {
  let position = mouse_position (& event);
  //eprintln!(" mouseup ");
  with_state_mut (| state | {
    let mut notes_changed = false;
    if let Some(drag_type) = state.drag_type() {
      //eprintln!(" {:?} ", drag_type);
      match drag_type {
        DragType::ClickNote (id) => state.selected = hashset!{id},
        DragType::MoveNotes {notes, exact_movement: _, rounded_movement, copying} => {
          let semitones = (rounded_movement [1]).round() as i32;
          let mut new_notes = Vec::new();
          for note in state.notes.iter_mut() {
            if notes.contains(&note.serial_number) {
              if copying {
                let mut new_note = note.note.clone();
                new_note.pitch += semitones;
                new_note.start_time += rounded_movement [0];
                new_notes.push(EditedNote::new_stealing (new_note, note));
              }
              else {
                note.note.pitch += semitones;
                note.note.start_time += rounded_movement [0];
              }
            }
          }
          state.notes.extend (new_notes);
          notes_changed = true;
        },
        DragType::DragSelect {notes, ..} => {
          state.selected = notes
        }
        _ => ()
      }
    }
    state.mouse.drag = None;
    if notes_changed {
      state.notes_changed(true);
    }
    else {
      state.update_elements();
    }
  });
}


fn main() {
  stdweb::initialize();
  println!("Hello from frontend (stdout)");
  eprintln!("Hello from frontend (stderr)");
  
  let receive_from_backend = |s:String| {
    println!("received message from backend stdout: {}", s);
    /*if let Ok(message) = serde_json::from_str(&s) {
      match message {
        MessageToFrontend::Print(text) => {
          println!("received from backend stdout as Print: {}", text);
        },
      }
    }
    else {
      println!("received invalid message from backend stdout: {}", s);
    }*/
  };
  
  js! {
const {spawn} = require("child_process");
backend = spawn("../target/debug/codecophony_editor_backend");

backend.stdout.on("data", function(data){@{receive_from_backend}(""+data)});
backend.stderr.on("data", function(data){
  console.log("received from backend stderr: "+data);
});

backend.on("close", (code)=>{
  console.log("backend exited with code "+code);
});
  }
  
  send_to_backend(&MessageToBackend::ReplacePlaybackScript(PlaybackScript {
    notes: vec![
      Note {start_time: 0.0, duration: 1.0, pitch: 64,},
      Note {start_time: 1.0, duration: 1.0, pitch: 66,},
      Note {start_time: 2.0, duration: 1.0, pitch: 67,},
      Note {start_time: 3.0, duration: 1.0, pitch: 69,},
      Note {start_time: 4.0, duration: 4.0, pitch: 71,},
    ],
    end: None,
    loop_back_to: None,
  }));
  
  send_to_backend(&MessageToBackend::RestartPlaybackAt (Some(0.0)));
  
  web::document().body().unwrap().add_event_listener (mouse_down);
  web::document().body().unwrap().add_event_listener (mouse_move);  
  web::document().body().unwrap().add_event_listener (mouse_up);
  
  with_state_mut(|state| {
    state.init_elements();
    for _ in 0..10 {
      state.notes.push (EditedNote::new (Note {
        start_time: rand::thread_rng().gen_range(0.0, 3.0),
        duration: 0.3,
        pitch: rand::thread_rng().gen_range(30, 80),
      }));
    }
    state.notes_changed(false);
  });
  
  js! {
    window.fs.readFile(window.autosave_path, "utf8", function(err, data) {
      if (!err) {
        @{load_json}(data);
      }
    });
    
    window.midi_input = new window.midi.input();
    window.midi_input.on ("message", function (delta_time, message) {
      console.log (message);
    });
    
    console.log("ports", window.midi_input.getPortCount());
    console.log("first", window.midi_input.getPortName(0));
    window.midi_input.openPort(0);
  }
  
  stdweb::event_loop();
}

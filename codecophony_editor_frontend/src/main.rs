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

use std::collections::HashSet;
use std::cell::RefCell;

use serde::Serialize;
use stdweb::web::event::{MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use stdweb::web::{self, IEventTarget, HtmlElement};
use stdweb::unstable::TryInto;
use stdweb::traits::*;
use nalgebra::Vector2;
use rand::prelude::*;


use shared::{MessageToBackend,MessageToFrontend,PlaybackScript,Note};

pub mod misc;
use misc::SerialNumber;

type Vector = Vector2 <f64>;
pub const PIXELS_PER_TIME: f64 = 30.0;
pub const PIXELS_PER_SEMITONE: f64 = 5.0;

pub mod edited_note {
use super::*;
use stdweb::Value;
use shared::Note;

pub struct EditedNote {
  pub note: Note,
  pub serial_number: SerialNumber,
  //pub selected: bool,
  element: Value,
}


impl EditedNote {
  pub fn new (note: Note)->EditedNote {
    EditedNote {
      note,
      serial_number: Default::default(),
      element: js!{ return ($("<div>", {class: "note", "data-handletype": "note"}).appendTo ($("#notes"))); }
    }
  }
  pub fn update_element(&self) {
    js!{
      let element =@{& self.element};
      element
        .width (@{self.note.duration*PIXELS_PER_TIME})
        .height(@{PIXELS_PER_SEMITONE})
        .attr("data-noteid", @{self.serial_number.0 as u32})
        .css({
          left:@{self.note.start_time*PIXELS_PER_TIME},
          bottom:@{self.note.pitch as f64*PIXELS_PER_SEMITONE}
        });
    }
  }
}


}

use edited_note::EditedNote;

#[derive (Debug)]
pub enum DragType {
  ClickNote (SerialNumber),
  DragSelect,
  MoveNotes (HashSet <SerialNumber>, Vector),
  ExtendNotes (HashSet <SerialNumber>, f64),
}

pub struct DragState {
  pub start_position: Vector,
  pub start_note: Option<SerialNumber>,
  pub start_handle_type: String,
  pub ever_moved_much: bool,
  //pub drag_type: DragType,
}

impl State {
  pub fn drag_type (&self)->Option<DragType> {
    self.mouse.drag.as_ref().map (|drag| {
    let movement = self.mouse.position - drag.start_position;
    
    if let Some(start_note) = drag.start_note {
      let what_notes = if self.selected.contains (& start_note) {
        self.selected.clone()
      }
      else {
        hashset!{start_note}
      };
      if !drag.ever_moved_much {
        DragType::ClickNote (start_note)
      }
      else if &drag.start_handle_type == "note" {
        DragType::MoveNotes (what_notes, movement)
      }
      else {
        DragType::ExtendNotes (what_notes, movement [0])
      }
    }
    else {
      DragType::DragSelect
    }
    })
  }
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct MouseState {
  pub drag: Option <DragState>,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub position: Vector,
}

thread_local! {
  static STATE: RefCell<State> = RefCell::default();
}

pub fn with_state<F: FnOnce(&State)->R, R> (callback: F)->R {
  STATE.with (| state | {
    let state = state.borrow();
    (callback)(&state)
  })
}
pub fn with_state_mut<F: FnOnce(&mut State)->R, R> (callback: F)->R {
  STATE.with (| state | {
    let mut state = state.borrow_mut();
    (callback)(&mut state)
  })
}

#[derive (Default)]
pub struct State {
  pub notes: Vec<EditedNote>,
  pub selected: HashSet <SerialNumber>,
  pub mouse: MouseState,
}

impl State {
  pub fn update_elements (&self) {
    for note in &self.notes {note.update_element()}
  }
  pub fn notes_changed (&self) {
    self.update_elements();
    send_to_backend(&MessageToBackend::ReplacePlaybackScript(PlaybackScript {
      notes: self.notes.iter().map(|a|a.note.clone()).collect(),
      end: None,
      loop_back_to: None,
    }));
    send_to_backend(&MessageToBackend::RestartPlaybackAt (Some(0.0)));
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
fn mouse_move_impl (position: Vector, raising: bool) {
  with_state_mut (| state | {
    state.mouse.position = position;
    if let Some(drag) = state.mouse.drag.as_mut() {
      if (position - drag.start_position).norm() > 5.0 {
        drag.ever_moved_much = true;
      }
    }
    if state.mouse.drag.is_some() && !raising {
      state.update_elements();
    }
  });
}
fn mouse_move (event: MouseMoveEvent) {
  mouse_move_impl (mouse_position (& event), false);
}
fn mouse_up (event: MouseUpEvent) {
  let position = mouse_position (& event);
  mouse_move_impl (position, true);
  //eprintln!(" mouseup ");
  with_state_mut (| state | {
    let mut notes_changed = false;
    if let Some(drag_type) = state.drag_type() {
      //eprintln!(" {:?} ", drag_type);
      match drag_type {
        DragType::ClickNote (id) => state.selected = hashset!{id},
        DragType::MoveNotes (notes, movement) => {
          let semitones = ((- movement [1])/PIXELS_PER_SEMITONE).round() as i32;
          for note in state.notes.iter_mut() {
            if notes.contains(&note.serial_number) {
              note.note.pitch += semitones;
              note.note.start_time += movement [0]/PIXELS_PER_TIME;
            }
          }
          notes_changed = true;
        },
        _ => ()
      }
    }
    state.mouse.drag = None;
    if notes_changed {
      state.notes_changed();
    }
    else {
      state.update_elements();
    }
  });
}

fn send_to_backend<T: Serialize> (send: &T) {
  let s = serde_json::to_string(send).unwrap();
  println!("Sending: {}", s);
  js!{
    backend.stdin.write(@{s});
    backend.stdin.write("\n");
  }
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
    for _ in 0..10 {
      state.notes.push (EditedNote::new (Note {
        start_time: rand::thread_rng().gen_range(0.0, 3.0),
        duration: 0.3,
        pitch: rand::thread_rng().gen_range(30, 80),
      }));
    }
    state.notes_changed();
  });
  
  stdweb::event_loop();
}

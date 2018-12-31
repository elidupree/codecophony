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
use misc::{SerialNumber, abs, min, max};

type Vector = Vector2 <f64>;
pub const PIXELS_PER_TIME: f64 = 100.0;
pub const PIXELS_PER_SEMITONE: f64 = 8.0;


pub struct NoteDrawingInfo <'a> {
  drag_type: Option <DragType>,
  selected: HashSet <SerialNumber>,
  state: & 'a State,
}

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
  pub fn update_element(&self, info: & NoteDrawingInfo) {
    let mut exact_pitch = self.note.pitch as f64;
    let mut rounded_pitch = exact_pitch;
    let mut exact_start = self.note.start_time;
    let mut rounded_start = exact_start;
    if let Some(drag_type) = &info.drag_type {match drag_type {
      DragType::MoveNotes {notes, exact_movement, rounded_movement} => {
        if notes.contains (& self.serial_number) {
          exact_pitch += exact_movement [1];
          rounded_pitch += rounded_movement [1];
          exact_start += exact_movement [0];
          rounded_start += rounded_movement [0];
        }
      },
      _=>(),
    }}
    rounded_pitch = rounded_pitch.round();
    let left = info.state.time_to_client (exact_start);
    let top = info.state.pitch_to_client (exact_pitch as f64 - 0.5);
    let width = self.note.duration * PIXELS_PER_TIME;
    let height = PIXELS_PER_SEMITONE;
    
    let color;
    let box_shadow;
    if exact_pitch == rounded_pitch && exact_start == rounded_start {
      color = "black";
      box_shadow = "none".to_string();
    } else {
      color = "rgba(0,0,0,0.5)";
      box_shadow = format! ("{}px {}px {}px {}",
        info.state.time_to_client (rounded_start) - info.state.time_to_client (exact_start),
        info.state.pitch_to_client (rounded_pitch) - info.state.pitch_to_client (exact_pitch),
        PIXELS_PER_SEMITONE/4.0,
        color,
      ) ;
    }
    
    js!{
      let element =@{& self.element};
      element
        .width (@{width})
        .height(@{height})
        .attr("data-noteid", @{self.serial_number.0 as u32})
        .css({
          left:@{left},
          top:@{top},
          "background-color": @{color},
          "box-shadow": @{box_shadow},
        });
    }
  }
}


}

use edited_note::EditedNote;

#[derive (Debug)]
pub enum DragType {
  ClickNote (SerialNumber),
  DragSelect {minima: Vector, maxima: Vector},
  MoveNotes {notes: HashSet <SerialNumber>, exact_movement: Vector, rounded_movement: Vector},
  ExtendNotes {notes: HashSet <SerialNumber>, exact_movement: f64, rounded_movement: f64},
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
    let exact_movement = self.client_to_music (self.mouse.position) - self.client_to_music (drag.start_position);
    
    if let Some(start_note) = drag.start_note {
      let notes = if self.selected.contains (& start_note) {
        self.selected.clone()
      }
      else {
        hashset!{start_note}
      };
      if !drag.ever_moved_much {
        DragType::ClickNote (start_note)
      }
      else if &drag.start_handle_type == "note" {
        let rounded_for_note = | note: SerialNumber | {
          let note = & self.get_note (note).unwrap().note;
          Vector::new (
            self.round_time (note.start_time + exact_movement [0]) - note.start_time,
            self.round_pitch (note.pitch as f64 + exact_movement [1]) - note.pitch as f64,
          )
        };
        let mut iterator = notes.iter().cloned();
        let mut rounded_movement = rounded_for_note (iterator.next().unwrap());
        for note in iterator {
          let rounded = rounded_for_note (note) ;
          for dimension in 0..2 {
            if abs (rounded [dimension] - exact_movement [dimension]) < abs (rounded_movement [dimension] - exact_movement [dimension]) {
              rounded_movement [dimension] = rounded [dimension];
            }
          }
        }
        DragType::MoveNotes {notes, exact_movement, rounded_movement}
      }
      else {
        DragType::ExtendNotes {notes, exact_movement: exact_movement[0], rounded_movement: exact_movement [0]}
      }
    }
    else {
      let music_start = self.client_to_music (drag.start_position);
      let music_stop = self.client_to_music (self.mouse.position) ;
      DragType::DragSelect {
        minima: Vector::new (
          min (music_start [0], music_stop [0]),
          min (music_start [1], music_stop [1]),
        ),
        maxima: Vector::new (
          max (music_start [0], music_stop [0]),
          max (music_start [1], music_stop [1]),
        ),
      }
    }
    })
  }
  
  pub fn get_note (&self, id: SerialNumber)->Option<& EditedNote> {
    self.notes.iter().find (| note | note.serial_number == id)
  }
  
  pub fn client_to_time (&self, client: f64)->f64 {
    client / PIXELS_PER_TIME
  }
  pub fn client_to_pitch (&self, client: f64)->f64 {
    (client / -PIXELS_PER_SEMITONE) + 101.5
  }
  pub fn time_to_client (&self, time: f64)->f64 {
    time * PIXELS_PER_TIME
  }
  pub fn pitch_to_client (&self, pitch: f64)->f64 {
    (pitch - 101.5) * -PIXELS_PER_SEMITONE
  }
  
  pub fn music_to_client (&self, music: Vector)->Vector {
    Vector::new (self.time_to_client (music [0]), self.pitch_to_client (music [1]))
  }
  pub fn client_to_music (&self, client: Vector)->Vector {
    Vector::new (self.client_to_time (client[0]), self.client_to_pitch (client[1]))
  }
  
  pub fn round_time (&self, time: f64)->f64 {
    (time*8.0).round()/8.0
  }
  pub fn round_pitch (&self, pitch: f64)->f64 {
    pitch.round()
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
    js!{ $("#notes").height (@{PIXELS_PER_SEMITONE*80.0 }); }
    let info = NoteDrawingInfo {
      drag_type: self.drag_type(),
      selected: self.selected.clone(),
      state: & self,
    };
    for note in &self.notes {note.update_element(& info)}
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
        DragType::MoveNotes {notes, exact_movement, rounded_movement} => {
          let semitones = (rounded_movement [1]).round() as i32;
          for note in state.notes.iter_mut() {
            if notes.contains(&note.serial_number) {
              note.note.pitch += semitones;
              note.note.start_time += rounded_movement [0];
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

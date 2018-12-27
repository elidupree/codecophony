#![recursion_limit="256"]

extern crate codecophony_editor_shared as shared;
#[macro_use] extern crate stdweb;
extern crate serde_json;
extern crate serde;
extern crate nalgebra;
extern crate rand;

use std::collections::HashSet;
use std::cell::RefCell;

use serde::Serialize;
use stdweb::web::event::{MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use stdweb::web::{self, IEventTarget};
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
      element: js!{ return ($("<div>", {class: "note"}).appendTo ($("#notes"))); }
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

pub struct DragState {
  start: Vector,
  
}

thread_local! {
  static STATE: RefCell<State> = RefCell::default();
}



#[derive (Default)]
pub struct State {
  pub notes: Vec<EditedNote>,
  pub selected: HashSet <SerialNumber>,
  pub drag: Option <DragState>,
}

impl State {
  pub fn notes_changed (&self) {
    for note in &self.notes {note.update_element()}
    send_to_backend(&MessageToBackend::ReplacePlaybackScript(PlaybackScript {
      notes: self.notes.iter().map(|a|a.note.clone()).collect(),
      end: None,
      loop_back_to: None,
    }));
    send_to_backend(&MessageToBackend::RestartPlaybackAt (Some(0.0)));
  }
}

fn mouse_down (event: MouseDownEvent) {
eprintln!(" {:?} ", "whatever");
  STATE.with (| state | {
    let mut state = state.borrow_mut();
    state.notes.push (EditedNote::new (Note {
      start_time: rand::thread_rng().gen_range(0.0, 3.0),
      duration: 0.3,
      pitch: rand::thread_rng().gen_range(30, 80),
    }));
    state.notes_changed()
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
  eprintln!(" {:?} ", "whatever");
  STATE.with (| state | {
    let mut state = state.borrow_mut();
    state.notes.push (EditedNote::new (Note {
      start_time: rand::thread_rng().gen_range(0.0, 3.0),
      duration: 0.3,
      pitch: rand::thread_rng().gen_range(30, 80),
    }));
    state.notes_changed()
  });
  stdweb::event_loop();
}

use std::cell::RefCell;
use std::collections::HashSet;

use misc::{SerialNumber, Vector, abs, min, max};
use edited_note::EditedNote;

#[derive (Default)]
pub struct State {
  pub notes: Vec<EditedNote>,
  pub selected: HashSet <SerialNumber>,
  pub mouse: MouseState,
}

#[derive (Derivative)]
#[derivative (Default)]
pub struct MouseState {
  pub drag: Option <DragState>,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub position: Vector,
  pub shift_key: bool,
  pub control_key: bool,
}



#[derive (Debug)]
pub enum DragType {
  ClickNote (SerialNumber),
  DragSelect {minima: Vector, maxima: Vector, notes: HashSet <SerialNumber>},
  MoveNotes {notes: HashSet <SerialNumber>, exact_movement: Vector, rounded_movement: Vector, copying: bool},
  ExtendNotes {notes: HashSet <SerialNumber>, exact_movement: f64, rounded_movement: f64},
}

pub struct DragState {
  pub start_position: Vector,
  pub start_note: Option<SerialNumber>,
  pub start_handle_type: String,
  pub ever_moved_much: bool,
  //pub drag_type: DragType,
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
        DragType::MoveNotes {notes, exact_movement, rounded_movement, copying: self.mouse.shift_key}
      }
      else {
        DragType::ExtendNotes {notes, exact_movement: exact_movement[0], rounded_movement: exact_movement [0]}
      }
    }
    else {
      let music_start = self.client_to_music (drag.start_position);
      let music_stop = self.client_to_music (self.mouse.position) ;
      let minima = Vector::new (
          min (music_start [0], music_stop [0]),
          min (music_start [1], music_stop [1]),
        );
      let maxima = Vector::new (
          max (music_start [0], music_stop [0]),
          max (music_start [1], music_stop [1]),
        );
      DragType::DragSelect {
        minima, maxima, notes: self.notes.iter().filter (| note | {
          note.note.start_time <= maxima [0] && note.note.start_time + note.note.duration >= minima [0] &&
          note.note.pitch as f64 - 0.5 <= maxima [1] && note.note.pitch as f64 + 0.5 >= minima [1]
        }).map (| note | note.serial_number).collect()
      }
    }
    })
  }
  
  pub fn get_note (&self, id: SerialNumber)->Option<& EditedNote> {
    self.notes.iter().find (| note | note.serial_number == id)
  }
  
  pub fn round_time (&self, time: f64)->f64 {
    (time*8.0).round()/8.0
  }
  pub fn round_pitch (&self, pitch: f64)->f64 {
    pitch.round()
  }
}



use serde::{Serialize, Deserialize};
use derivative::Derivative;
use std::collections::{HashSet, HashMap};
use uuid::Uuid;
use nalgebra::Vector2;
use maplit::hashset;


pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}
pub fn abs (first: f64)->f64 {if first < 0.0 {-first} else {first}}

pub type Vector = Vector2 <f64>;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct Note {
  pub start_time: f64,
  pub duration: f64,
  // in MIDI pitch scale, but allowing fractional values
  pub pitch: f64,
  #[serde (default = "Uuid::new_v4")]
  #[derivative (Default (value=" Uuid::new_v4()"))]
  pub id: Uuid,
}

impl Note {
  pub fn end_time(&self)->f64 {self.start_time + self.duration}
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Chunk {
  pub notes: Vec<Note>,
  
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Default)]
pub struct View {
  pub selected: HashSet <Uuid>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Project {
  pub chunks: HashMap <Uuid, Chunk>,
  pub views: HashMap <String, View>,
  #[serde (skip)]
  pub mouse: MouseState,
}


#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct MousePosition {
  pub view: String,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub client_position: Vector,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub music_position: Vector,
  pub target: MouseTarget,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum MouseTarget {
  #[derivative (Default)]
  None,
  Note {id: Uuid, region: NoteRegion },
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub enum NoteRegion {
  #[derivative (Default)]
  Body,
  Tail
}



#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct MouseState {
  
  pub drag: Option <DragState>,
  pub position: MousePosition,
  pub shift_key: bool,
  pub control_key: bool,
}



#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum DragType {
  ClickNote (Uuid),
  DragSelect {minima: Vector, maxima: Vector, notes: HashSet <Uuid>},
  MoveNotes {notes: HashSet <Uuid>, exact_movement: Vector, rounded_movement: Vector, copying: bool},
  ExtendNotes {notes: HashSet <Uuid>, exact_movement: f64, rounded_movement: f64},
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct DragState {
  pub start_position: MousePosition,
  pub ever_moved_much: bool,
}

impl Project {
  pub fn drag_type (&self)->Option <DragType> {
    if let Some(drag) = self.mouse.drag.as_ref() {Some ({
      let start = drag.start_position.music_position;
      let current = self.mouse.position.music_position;
      let exact_movement = current - start;
      let view = self.views.get (& drag.start_position.view).expect ("shouldn't be able to drag from a view that doesn't exist");
      
      match &drag.start_position.target {
        MouseTarget::None => {
          if self.mouse.position.view != drag.start_position.view {
            return None
          }
          let minima = Vector::new (min (start [0], current [0]), min (start [1], current [1]));
          let maxima = Vector::new (max (start [0], current [0]), max (start [1], current [1]));
          let notes = self.chunks.values().flat_map (| chunk | chunk.notes.iter().filter_map (| note | {
            if note.start_time <= maxima [0] && note.end_time() >= minima [0] && note.pitch - 0.5 <= maxima [1] && note.pitch + 0.5 >= minima [1] {
              Some (note.id)
            }
            else {None}
          }));
          DragType::DragSelect {
            minima, maxima, notes: notes.collect()
          }
        }
        MouseTarget::Note {id: start_note_id, region} => {
          if !drag.ever_moved_much {
            DragType::ClickNote (start_note_id.clone())
          }
          else {
            let affected_notes = if view.selected.contains (& start_note_id) {view.selected.clone()} else {hashset! {start_note_id.clone()}};
            match region {
              NoteRegion::Tail => DragType::ExtendNotes {notes: affected_notes, exact_movement: exact_movement [0], rounded_movement: exact_movement [0]},
              NoteRegion::Body => {
                /*let rounded_for_note = | id: Uuid | {
                  
                };*/
                DragType::MoveNotes {notes: affected_notes, exact_movement, rounded_movement: exact_movement, copying: self.mouse.shift_key}
              }
            }
          }
        }
      }
    })}
    else {
      None
    }
  }
}


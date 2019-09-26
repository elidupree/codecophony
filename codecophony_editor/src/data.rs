use serde::{Serialize, Deserialize};
use derivative::Derivative;
use std::collections::{HashSet, HashMap};
use uuid::Uuid;
use nalgebra::Vector2;



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
  pub uuid: Uuid,
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
  Note { uuid: Uuid, region: NoteRegion },
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
  pub view: String,
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


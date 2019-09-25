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
  #[serde (skip)]
  pub mouse: MouseState,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Project {
  pub chunks: HashMap <Uuid, Chunk>,
  pub views: HashMap <String, View>,
}



#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Derivative)]
#[derivative (Default)]
pub struct MouseState {
  pub drag: Option <DragState>,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub client_position: Vector,
  #[derivative (Default (value = "Vector::new (0.0, 0.0)"))]
  pub music_position: Vector,
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
  pub start_client_position: Vector,
  pub start_music_position: Vector,
  pub start_note: Option<Uuid>,
  pub start_handle_type: String,
  pub ever_moved_much: bool,
}


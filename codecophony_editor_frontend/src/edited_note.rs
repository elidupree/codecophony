use std::mem;

use stdweb::Value;

use shared::Note;

use misc::{SerialNumber};
use data::{State, DragType};
use draw::{PIXELS_PER_SEMITONE, PIXELS_PER_TIME};

pub struct NoteDrawingInfo <'a> {
  pub drag_type: Option <DragType>,
  pub state: & 'a State,
}

pub struct EditedNote {
  pub note: Note,
  pub serial_number: SerialNumber,
  //pub selected: bool,
  element: Value,
}


impl EditedNote {
  fn new_element() -> Value {
    let result: Value = js!{ return ($("<div>", {class: "note", "data-handletype": "note"}).appendTo ($("#notes"))); };
    result
  }
  pub fn new (note: Note)->EditedNote {
    EditedNote {
      note,
      serial_number: Default::default(),
      element: Self::new_element()
    }
  }
  pub fn new_stealing (note: Note, steal_from: &mut EditedNote)->EditedNote {
    let element = mem::replace(&mut steal_from.element, Self::new_element());
    EditedNote {
      note,
      serial_number: Default::default(),
      element
    }
  }
  pub fn update_element(&self, info: & NoteDrawingInfo) {
    let mut exact_pitch = self.note.pitch as f64;
    let mut rounded_pitch = exact_pitch;
    let mut exact_start = self.note.start_time;
    let mut rounded_start = exact_start;
    let mut transition = "all 0.2s ease-out";
    let selected = info.state.selected.contains (& self.serial_number);
    let mut selecting = false;
    if let Some(drag_type) = &info.drag_type {match drag_type {
      DragType::MoveNotes {notes, exact_movement, rounded_movement, copying} => {
        if notes.contains (& self.serial_number) {
          exact_pitch += exact_movement [1];
          rounded_pitch += rounded_movement [1];
          exact_start += exact_movement [0];
          rounded_start += rounded_movement [0];
          transition = "none";
        }
      },
      DragType::DragSelect {notes,..} => {
        if notes.contains (& self.serial_number) {
          selecting = true;
        }
      },
      _=>(),
    }}
    rounded_pitch = rounded_pitch.round();
    let left = info.state.time_to_client (exact_start);
    let top = info.state.pitch_to_client (exact_pitch as f64 + 0.5);
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
          transition:@{transition},
        });
      if (@{selected}) {element.addClass ("selected");} else {element.removeClass ("selected");}
      if (@{selecting}) {element.addClass ("selecting");} else {element.removeClass ("selecting");}
    }
  }
}

impl Drop for EditedNote {
  fn drop(&mut self) {
    js!{@{&self.element}.remove();}
  }
}



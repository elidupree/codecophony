pub const PIXELS_PER_TIME: f64 = 100.0;
pub const PIXELS_PER_SEMITONE: f64 = 8.0;

use misc::Vector;
use data::{State, DragType};
use edited_note::NoteDrawingInfo;

impl State {
  
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


  pub fn update_elements (&self) {
    js!{ $("#notes").height (@{PIXELS_PER_SEMITONE*80.0 }); }
    let info = NoteDrawingInfo {
      drag_type: self.drag_type(),
      selected: self.selected.clone(),
      state: & self,
    };
    js!{ $(".drag_select").remove() ;}
    if let Some(DragType::DragSelect {minima, maxima, ..}) = info.drag_type {
      let minima = self.music_to_client (minima);
      let maxima = self.music_to_client (maxima);
      let size = maxima - minima;
      js!{ $("<div>", {class: "drag_select"}).appendTo ($("#notes")).css ({
        left:@{minima [0]},
        top:@{maxima [1]},
        width:@{size[0]},
        height:@{-size[1]},
      });}
    }
    for note in &self.notes {note.update_element(& info)}
  }
  
  pub fn init_elements (&self) {
    for octave in 0..10 {
      for (index, black) in vec![false, true, false, false, true, false, true, false, false, true, false, true].into_iter().enumerate() {
        let pitch = (octave*12 + index + 21) as f64;
        if black {
          js!{
            $("#notes").append ($("<div>", {class: "key"}).css({top: @{self.pitch_to_client (pitch+0.5)}, height:@{PIXELS_PER_SEMITONE}, "background-color": "#ddd"}));
          }
        }
      }
    }
  }
}

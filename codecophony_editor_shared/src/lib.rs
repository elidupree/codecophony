#[macro_use] extern crate serde_derive;
extern crate serde;

#[derive(Serialize, Deserialize)]
pub enum MessageToBackend {
  ReplacePlaybackScript(PlaybackScript),
  RestartPlaybackAt(Option<f64>),
}

#[derive(Serialize, Deserialize)]
pub enum MessageToFrontend {
  PlaybackStalledAt(Option<f64>),
  PlaybackResumed,
}


#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Note {
  pub start_time: f64,
  pub duration: f64,
  pub pitch: i32,
}

impl Note {
  pub fn end_time(&self)->f64 {self.start_time + self.duration}
}


#[derive(Serialize, Deserialize, Default)]
pub struct PlaybackScript {
  pub notes: Vec<Note>,
  pub end: Option<f64>,
  pub loop_back_to: Option<f64>,
}

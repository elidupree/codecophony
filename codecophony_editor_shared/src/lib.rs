#[macro_use] extern crate serde_derive;
extern crate serde;

#[derive(Serialize, Deserialize)]
pub enum MessageToBackend {
  Echo(String),
}

#[derive(Serialize, Deserialize)]
pub enum MessageToFrontend {
  Print(String),
}

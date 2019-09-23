use parking_lot::Mutex;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket::response::NamedFile;
use rocket::State;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
//use rocket::response::content::Json;
use rocket_contrib::json::Json;
use typed_html::dom::DOMTree;
use typed_html::elements::FlowContent;
use typed_html::{html, text};
use std::sync::mpsc::{Sender, Receiver, channel};
use crate::rendering::{Note, PlaybackScript, MessageToRenderThread};


pub type Element = Box<dyn FlowContent<String>>;



#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MessageToBackend {

}

pub struct ApplicationState {
  send_to_render_thread: Sender<MessageToRenderThread>,  
}

impl ApplicationState {

          

}

  

pub struct RocketState {
  application_state: Arc<Mutex<ApplicationState>>,
  root_path: PathBuf,
}

#[get("/views/<view>")]
fn view(view: String, rocket_state: State<RocketState>) -> Option<NamedFile> {
  NamedFile::open(rocket_state.root_path.join("static/view.html")).ok()
}

#[post("/view_content/<view>", data = "<message>")]
fn content(view: String, message: Json<MessageToBackend>, rocket_state: State<RocketState>) -> String {
  let application_state = rocket_state.application_state.lock();

  let document: DOMTree<String> = html! {
    <div id="content">
      {text!("foo")}
    </div>
  };
  document.to_string()
}

#[get("/media/<file..>")]
fn media(file: PathBuf, rocket_state: State<RocketState>) -> Option<NamedFile> {
  NamedFile::open(rocket_state.root_path.join("static/media/").join(file)).ok()
}

pub fn run(project_dir: PathBuf) {
  let mut send_to_render_thread = crate::rendering::spawn_render_thread();
  send_to_render_thread.send(MessageToRenderThread::RestartPlaybackAt(Some(0.0))).unwrap();
  send_to_render_thread.send(MessageToRenderThread::ReplaceScript(PlaybackScript {
    notes: vec![
      Note {start_time: 0.0, duration: 1.0, pitch: 64,},
      Note {start_time: 1.0, duration: 1.0, pitch: 66,},
      Note {start_time: 2.0, duration: 1.0, pitch: 67,},
      Note {start_time: 3.0, duration: 1.0, pitch: 69,},
      Note {start_time: 4.0, duration: 4.0, pitch: 71,},
    ],
    end: None,
    loop_back_to: None,
  })).unwrap();
  let mut application_state = ApplicationState {
    send_to_render_thread
  };

  /*if let Ok(file) = std::fs::File::open(root_path.join("last_state.json")) {
    if let Ok(state) = serde_json::from_reader(std::io::BufReader::new(file)) {
      application_state.set_state(state);
    }
  }*/

  let application_state = Arc::new(Mutex::new(application_state));  

  rocket::custom(
    Config::build(Environment::Development)
      .address("localhost")
      .port(3508)
      .log_level(LoggingLevel::Off)
      .unwrap(),
  )
  .mount("/", routes![view, media, content])
  .manage(RocketState {
    application_state,
    root_path: PathBuf::from("."),
  })
  .launch();
}

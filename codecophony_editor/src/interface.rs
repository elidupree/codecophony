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
use std::sync::mpsc::{Sender};
use std::collections::HashMap;
use crate::rendering::{PlaybackScript, MessageToRenderThread};
use crate::data::{Vector, Project, Chunk, Note, View, MousePosition, MouseTarget, DragState, DragType, NoteRegion};
use maplit::{hashset, hashmap};
use uuid::Uuid;


pub type Element = Box<dyn FlowContent<String>>;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum MouseEventType {
  #[serde (rename = "mousedown")]
  MouseDown,
  #[serde (rename = "mousemove")]
  MouseMove,
  #[serde (rename = "mouseup")]
  MouseUp,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Action {
  MouseEvent {
            position: MousePosition,
            shift_key: bool,
            control_key: bool,
            event_type: MouseEventType,
          },
}

pub struct ApplicationState {
  project: Project,
  send_to_render_thread: Sender<MessageToRenderThread>,
}

pub const PIXELS_PER_TIME: f64 = 100.0;
pub const PIXELS_PER_SEMITONE: f64 = 8.0;
impl View {
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

  fn rendered (&self, project: & Project)->Element {
    let notes = project.chunks.values().flat_map (| chunk | {
      chunk.notes.iter().map (| note | {
        let style = format! ("
          left: {}px;
          top: {}px;
          width: {}px;
          height: {}px;
        ", self.time_to_client (note.start_time), self.pitch_to_client (note.pitch + 0.5), note.duration*PIXELS_PER_TIME, PIXELS_PER_SEMITONE);
        html! {
          <div id={typed_html::types::Id::new (format!("note_{}", note.id))} class="note" data-target={serde_json::to_string (& MouseTarget::Note {id: note.id, region: NoteRegion::Body}).unwrap()} style={style}>
            
          </div>
        }
      })
    });
    
    html! {
      <div class="view">
        {notes}
      </div>
    }
  }
          

}
  

pub struct RocketState {
  application_state: Arc<Mutex<ApplicationState>>,
  root_path: PathBuf,
}

#[get("/views/<_view>")]
fn view(_view: String, rocket_state: State<RocketState>) -> Option<NamedFile> {
  NamedFile::open(rocket_state.root_path.join("static/view.html")).ok()
}

#[get("/views/<view>/content")]
fn content(view: String, rocket_state: State<RocketState>) -> String {
  let state = rocket_state.application_state.lock();
  let default_view = View::default();
  let view = state.project.views.get(&view).unwrap_or(&default_view);
  let document: DOMTree<String> = html! {
    <div id="content">
      {view.rendered (& state.project)}
    </div>
  };
  document.to_string()
}

#[post("/views/<view>/action", data = "<action>")]
fn action(view: String, action: Json<Action>, rocket_state: State<RocketState>) {
  let mut state = rocket_state.application_state.lock();
  //dbg!((& view, & action));
  let project = &mut state.project;
  match action.into_inner() {
    Action::MouseEvent {mut position, shift_key, control_key, event_type} => {
      assert_eq!(position.view, view);
      let view = project.views.entry (position.view.clone()).or_default();
      position.music_position = view.client_to_music (position.client_position);
      project.mouse.position = position.clone();
      project.mouse.shift_key = shift_key;
      project.mouse.control_key = control_key;
      
      if let Some(drag) = project.mouse.drag.as_mut() {
        if (position.client_position - drag.start_position.client_position).norm() >5.0 {
          drag.ever_moved_much = true;
        }
      }
      
      
      
      match event_type {
        MouseEventType::MouseMove => {}
        MouseEventType::MouseDown => {
          eprintln!("Drag began at {:?} ", position);
          project.mouse.drag = Some (DragState {
            start_position: position,
            ever_moved_much: false,
          });
        }
        MouseEventType::MouseUp => {
          eprintln!("Drag ended at {:?} ", position);
          if let Some(drag_type) = project.drag_type() {
            eprintln!("  as {:?} ", drag_type);
            let view = project.views.get_mut (& position.view).unwrap();
            match drag_type {
              DragType::ClickNote (id) => view.selected = hashset! {id},
              DragType::DragSelect {notes, ..} => {
                view.selected = notes;
              }
              DragType::ExtendNotes {notes, exact_movement:_, rounded_movement} => {
                unimplemented!()
              }
              DragType::MoveNotes {notes, exact_movement:_, rounded_movement, copying} => {
                let mut new_notes = Vec::new();
                for (chunk_id, chunk) in &mut project.chunks {
                  for note in &mut chunk.notes {
                    if notes.contains (& note.id) {
                      if copying {
                        new_notes.push ((chunk_id.clone(), Note {id: Uuid::new_v4(), ..note.clone()}));
                      }
                      note.pitch += rounded_movement [1];
                      note.start_time += rounded_movement [0];
                    }
                  }
                }
                for (chunk_id, note) in new_notes {
                  project.chunks.get_mut (& chunk_id).unwrap().notes.push (note) ;
                }
              }
            }
          }
          project.mouse.drag = None;
        }
      }
    }
  }
}

#[get("/media/<file..>")]
fn media(file: PathBuf, rocket_state: State<RocketState>) -> Option<NamedFile> {
  NamedFile::open(rocket_state.root_path.join("static/media/").join(file)).ok()
}

pub fn run(project_dir: PathBuf) {
  /*eprintln!("{}", serde_json::to_string(&Action::MouseEvent{
      position: MousePosition {
        client_position: Vector::new(1.0, 1.5),
        music_position: Vector::new(1.0, 1.5),
        target: MouseTarget::None,
      },
      shift_key: false, control_key: false, event_type: MouseEventType::MouseMove}
        ).unwrap());*/
  let chunk_id = Uuid::new_v4();
  let project = Project {
    chunks: hashmap!{chunk_id => Chunk {
    notes: vec![
      Note {start_time: 0.0, duration: 1.0, pitch: 64.0,..Default::default()},
      Note {start_time: 1.0, duration: 1.0, pitch: 66.0,..Default::default()},
      Note {start_time: 2.0, duration: 1.0, pitch: 67.0,..Default::default()},
      Note {start_time: 3.0, duration: 1.0, pitch: 69.0,..Default::default()},
      Note {start_time: 4.0, duration: 4.0, pitch: 71.0,..Default::default()},
    ]
    }},
    views: HashMap::new(),
    mouse: Default::default(),
  };

  let send_to_render_thread = crate::rendering::spawn_render_thread();
  send_to_render_thread.send(MessageToRenderThread::RestartPlaybackAt(Some(0.0))).unwrap();
  send_to_render_thread.send(MessageToRenderThread::ReplaceScript(PlaybackScript {
    notes: project.chunks.values().flat_map (| chunk | chunk.notes. iter().cloned()).collect(),
    end: None,
    loop_back_to: None,
  })).unwrap();
  let application_state = ApplicationState {
    send_to_render_thread, project
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
      .port(3413)
      .log_level(LoggingLevel::Critical)
      .unwrap(),
  )
  .mount("/", routes![view, media, content, action])
  .manage(RocketState {
    application_state,
    root_path: PathBuf::from("."),
  })
  .launch();
}

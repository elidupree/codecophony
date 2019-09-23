#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use std::path::PathBuf;

mod interface;
mod rendering;

fn main() {
  let arguments: Vec<String> = std::env::args().collect();
  if let Some(dir) = arguments.get(1) {
    interface::run(PathBuf::from(dir.clone()));
  }
  else {
    println!("Usage: codecophony_editor path-to-project-dir")
  }
}

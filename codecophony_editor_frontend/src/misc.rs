use std::cell::Cell;

#[derive (Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SerialNumber (pub u64);

impl Default for SerialNumber {
  fn default()->Self {
    thread_local! {static NEXT_SERIAL_NUMBER: Cell<u64> = Cell::new (0);}
    NEXT_SERIAL_NUMBER.with (| next | {
      let this = next.get();
      next.set (this + 1);
      SerialNumber (this)
    })
  }
}


pub fn min (first: f64, second: f64)->f64 {if first < second {first} else {second}}
pub fn max (first: f64, second: f64)->f64 {if first > second {first} else {second}}
pub fn abs (first: f64)->f64 {if first < 0.0 {-first} else {first}}

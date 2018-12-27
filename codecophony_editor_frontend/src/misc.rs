use std::cell::Cell;

#[derive (Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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

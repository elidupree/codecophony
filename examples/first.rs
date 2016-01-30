extern crate codecophony;
extern crate hound;

use codecophony::*;

fn main() {
  let manual = scrawl_MIDI_notes(
                            "transpose 57 velocity 100 instrument 61
12 and 15 and 19 5 8 step 0.5 5 8 10
12 quiet sustain 17 quiet sustain 20 step \
                             1 5 step 0.5 7 step 2.5
finish release 17 release 20
");
  let notes = Notes::combining(&[manual.clone(),
                                 manual.translated(8.0),
                                 manual.translated(16.0).transposed(7),
                                 manual.translated(24.0).transposed(7)])
                .scaled(0.25);


  // add (0.0, 0); add (1.5, 5); add (2.0, 7); add (3.0, 11); add (4.0, 12);

  let music = notes.render_default(44100);

  let spec = hound::WavSpec {
    channels: 1,
    sample_rate: 44100,
    bits_per_sample: 16,
  };
  let mut writer = hound::WavWriter::create("output.wav", spec).unwrap();
  for t in music.samples.iter() {
    writer.write_sample(*t as i16).unwrap();

  }
}

#[macro_use] extern crate serde_derive;

use codecophony::*;
use dsp::Frame;

//mod sandbox;
mod procedural_generation_1;

//use sandbox::{SAMPLE_HZ, CHANNELS, Output};
use self::procedural_generation_1::{SAMPLE_HZ, CHANNELS, Output};

fn main() {
//write_eggs ();
//write_palette ();
  //loop {::std::thread::sleep(::std::time::Duration::from_millis(16));}
  
  // /*
  //let (notes,_) = sandbox::current_playground();
  let notes = procedural_generation_1::generate_music();
  
  let spec = hound::WavSpec {
      channels: CHANNELS as u16,
      sample_rate: SAMPLE_HZ as u32,
      bits_per_sample: 16,
      sample_format: hound::SampleFormat::Int,
    };
    let data = PositionedSequence::<[Output;CHANNELS],Vec<[Output;CHANNELS]>>::rendered_from(&*notes, SAMPLE_HZ);
    let mut writer = hound::WavWriter::create("interval_optimized.wav", spec).unwrap();
    {
    for frame in data.frames.iter() {
      for sample in frame.channels() {
        writer.write_sample((sample*2f32.powi(15)) as i16).unwrap();
      }
    }
    }
    writer.finalize().unwrap();
  //*/
    
  //sandbox::current_watcher();
}

/*


[Incomplete lyrics for a song I was writing.

CONTENT WARNING: diet culture stuff]



Eating eggs gives you cancer, eating bacon makes you die
you'll get diabetes if you eat a single slice of pie
I'm concerned for your health, you're not eating very well,
if you have just one more sandwich then you might go straight to hell






Just trust me, I'm a doctor and you need to change your weight
if your BMI is less than 95 or more than 8
if you take my instructions dieting will be a breeze
don't eat carbs and don't eat fats and don't eat any calories
eating eggs, etc. It's clear your excess weight has caused you ills of every kind
it's why you fell and broke your arm – it's why you're colorblind



You must be, very careful of the things that you consume
Anything that has cholesterol will surely spell your doom
only trust ingredients from recipes used by your mom
if your food is full of chemicals it's probably a bomb
eating eggs, etc.

If they let you eat what feels good they are hopelessly naïve
I'm telling you the rules and I'm the one you should believe
you must only eat just enough to make you stay alive
there's totally no need for you to heal or grow or thrive

If you eat a chicken dinner then you'll have a heart attack
you'll get morbidly obese if you indulge a midnight snack
and, all the effort you put into following the way
will be wasted if you eat more than 10 calories a day
eating eggs, etc.

[Chorus repeats; voiceover:]



Eating eggs, etc.
so don't take any of that bad diet advice

*/

/*
fn write_eggs () {
  let mut main_melody = scrawl_MIDI_notes(
                            "transpose 57 velocity 100 instrument 55
        0 step 1 5 7 step 2 7 step 1 5 7 0 0 5 7 7 7 5 7 step 2
        0 step 1 5 7 5 7 5 7 5 7 5 7 5 7 5 7 step 2
          0 step 1 5 7 step 2 7 step 1 5 7 0 0 5 7 7 7 5 7 step 2
          0 step 1 5 7 step 2 7 step 1 5 7 0 0 5 7 7 10 5 7 step 2 finish
");
for note in main_melody.iter_mut () {if note.basics.start % 4.0 == 2.0 {note.renderer.velocity = 127;}}
let chorus_harmony = scrawl_MIDI_notes ("transpose 45 instrument 31 velocity 40
at 2 0 and 4 and 7 and 12
at 18 0 and 5 and 9 and 12 and 17
at 34 0 and 4 and 7 and 12 
at 50 2 and 7 and 17 and 22 
at 64 finish ");
let chorus_beat_part = scrawl_MIDI_notes ("velocity 100 percussion 35 step 2 35 and 38 finish");
let mut chorus_beat = Notes::new ();
for offset in 0..16 {chorus_beat.add (& chorus_beat_part.translated (offset as f64*4.0));}
let mut prechorus_beat = Notes::new ();
for offset in 0..8 {prechorus_beat.add (& scrawl_MIDI_notes ("velocity 100 percussion at 2 35 step 4 38 finish").translated (offset as f64*8.0));}

let segment_length = 64.0;

/*

I wrote this verse with a boring rhythm (it should really have dotted notes and stuff) but I'm going to leave the customization for when I actually sing it.


*/
let mut believe_melody = scrawl_MIDI_notes ("transpose 57 velocity 100 instrument 55

5 5 strong 7 2 2 0 2 2 5 7 strong 5 0 0 -2 0 advance 2
transpose 53 advance 1 strong 7 2 2 2 0 2 2 strong 5 0 0 0 0 -2 0 advance 2
transpose 57 -2 0 0 2 2 4 4 5 5 strong 7 0 0 -2 0 advance 3
transpose 53 -2 0 0 2 2 4 4 5 5 strong 7 0 0 -2 0 advance 3");

let standard_chorus_speed = 30.0/170.0;
let standard_chorus_length = segment_length*standard_chorus_speed;
let standard_prechorus_speed = 35.0/170.0;
let standard_prechorus_length = segment_length*standard_prechorus_speed;
let standard_verse_speed = 40.0/170.0;
let standard_verse_length = segment_length*standard_verse_speed;

  let standard_chorus = Notes::combining(&[main_melody.clone (), chorus_harmony.clone (), chorus_beat.clone ()])
                .scaled(standard_chorus_speed);
let standard_prechorus = Notes::combining (& [main_melody.clone (), chorus_harmony.clone (), prechorus_beat.clone ()]).scaled (standard_prechorus_speed);

let mut now = 0.0;
let opening_chorus_start = now;
let mut notes = Notes::new ();
notes.add (& standard_chorus.translated (opening_chorus_start));
now += standard_chorus_length;
let believe_start = now;
notes.add (& believe_melody.scaled (standard_verse_speed).translated (now));
now += standard_verse_length;
notes.add (& standard_prechorus.translated (now));
now += standard_prechorus_length;
notes.add (& standard_chorus.translated (now));

  let music = notes.render_default(44100);

  let spec = hound::WavSpec {
    channels: 1,
    sample_rate: 44100,
    bits_per_sample: 16,
  };
  let mut writer = hound::WavWriter::create("eggs.wav", spec).unwrap();
  for t in music.samples.iter() {
    writer.write_sample(*t as i16).unwrap();

  }
}

fn write_palette () {
let mut notes = Notes::new ();
for offset in 1..129 {
notes.add (& scrawl_MIDI_notes (& ("instrument ".to_string () + & offset.to_string () + & " 64".to_string ())).translated ((offset + (offset -1)/4) as f64));
}
for offset in 35.. 82 {
notes.add (& scrawl_MIDI_notes (& (" percussion ".to_string () + & offset.to_string ())).translated ((180+ offset + (offset - 35)/4) as f64));
} 

  let music = notes.render_default(44100);

  let spec = hound::WavSpec {
    channels: 1,
    sample_rate: 44100,
    bits_per_sample: 16,
  };
  let mut writer = hound::WavWriter::create("palette.wav", spec).unwrap();
  for t in music.samples.iter() {
    writer.write_sample(*t as i16).unwrap();

  }

}*/

extern crate cpal;
extern crate hound;

use std::cmp::{min, max};
use std::collections::HashMap;
use std::str::FromStr;

type Position = i32;
type Sample = i32;
type Semitones = i32;
const semitone_ratio: f64 = (1.0594631f64);

#[derive (Clone)]
struct Sequence {
start: Position,
samples: Vec< Sample>,
}

trait Note {
fn render (& self, sample_rate: Position)->Sequence;
fn transpose (& mut self, semitones: Semitones) {}
}

struct hack_note {
start: f64,
frequency:f64,
amplitude: f64,
}

impl Note for hack_note {
fn render (& self, sample_rate: Position)->Sequence {
let mut samples: Vec< Sample> = Vec::new ();
for time in 0..(sample_rate/4) {
samples.push ((self.amplitude*(self.frequency*time as f64*(std::f64::consts::PI*2.0)/sample_rate as f64).sin ()) as Sample);
}
Sequence {start: (self.start*sample_rate as f64) as Position, samples: samples}
}
fn transpose (&mut self, semitones: Semitones) {
self.frequency *= semitone_ratio.powi (semitones);
}
}

//TODO: take a less specific "collection of sequences" argument type
fn merge (sequences: & Vec< Sequence>)->Sequence {
let mut minimum = Position::max_value ();
let mut maximum = Position::min_value ();
for sequence in sequences {
minimum = min (minimum, sequence.start);
maximum = max (maximum, sequence.start + sequence.samples.len() as Position);
}
let mut samples :  Vec< Sample> =vec! [0; (maximum - minimum) as usize];
for sequence in sequences {
for (index, sample) in sequence.samples.iter ().enumerate () {
samples [(sequence.start - minimum ) as usize + index] += *sample;
}
}
Sequence {start: minimum, samples: samples}
}

fn interpret_scrawl <note_factory_type:FnMut (f64, f64, Semitones)> (note_factory: &mut note_factory_type, scrawl: &str) {
let mut now = 0.0f64;
let mut instructions = scrawl.split_whitespace ();
#[derive (Clone, Copy)]
struct note_info {
beginning: f64,
}
let mut sustained_notes = HashMap::new ();
let mut latest_notes = HashMap::new ();
let mut step_size = 1.0f64;
let consume_number = | instructions: &mut std::str::SplitWhitespace | f64::from_str (instructions.next ().unwrap ()).unwrap ();
let consume_semitones =| instructions: &mut std::str::SplitWhitespace | Semitones::from_str (instructions.next ().unwrap ()).unwrap ();
let finish_note = | now, semitones, info: note_info, note_factory: &mut note_factory_type | {note_factory (info.beginning, now, semitones);};
let finish_notes = | now: &mut f64, step_size, latest_notes: &mut HashMap <Semitones, note_info>, note_factory: &mut note_factory_type | {
let last_begin = latest_notes.values ().fold (- 900000000.0f64, | max, info: &note_info | if info.beginning >max {info.beginning} else {max});
let step_end = last_begin + step_size;
if step_end >*now {*now = step_end};

for (semitones, info) in latest_notes .iter () {
finish_note (*now, *semitones, *info, note_factory);
}
latest_notes.clear ();
};
let do_note = | now, semitones, container: &mut HashMap <Semitones, note_info> | {
container.insert (semitones, note_info {beginning: now});
};
loop {
match instructions.next () {
None => break,
Some (instruction) => {
match Semitones::from_str (instruction) {
Err (_) => match instruction {
"at" => {
let time = consume_number (&mut instructions);
if time <now {assert! (latest_notes.is_empty ())}
now = time;
},
"advance" => {
let time = consume_number (&mut instructions);
if time <0.0 {assert! (latest_notes.is_empty ())}
now += time;
},
"and" => do_note (now, consume_semitones (&mut instructions), &mut latest_notes),
"sustain" => do_note (now, consume_semitones (&mut instructions), &mut sustained_notes),
"release" => {
let semitones = consume_semitones (&mut instructions);
finish_note (now, semitones, sustained_notes.remove (&semitones).unwrap (), note_factory);
},
"step" => step_size = consume_number (&mut instructions),
"finish" => finish_notes (&mut now, step_size, &mut latest_notes, note_factory),
_=> (),
},
Ok (semitones) => {
finish_notes (&mut now, step_size, &mut latest_notes, note_factory);
do_note (now, semitones, &mut latest_notes)
},
}
}
}
}
}



fn main() {
    let endpoint = cpal::get_default_endpoint().unwrap();
    let format = endpoint.get_supported_formats_list().unwrap().next().unwrap();
    let mut channel = cpal::Voice::new(&endpoint, &format).unwrap();
    
    println!( "sample rate is {}", format.samples_rate.0);
    let mut notes: Vec< hack_note> = Vec::new ();

{
let mut add = | time: f64, pitch | {notes.push (
	hack_note {start: time/4.0, frequency: 440.0*semitone_ratio.powi (pitch), amplitude: 4000.0,})};

let mut note_factory = | start: f64, end: f64, semitones: Semitones | {add (start, semitones - 12);};
interpret_scrawl (&mut note_factory, "12 and 15 and 19 5 8 step 0.5 5 8 10 12 step 1.5 5 step 0.5 7 advance 2 finish");
//add (0.0, 0); add (1.5, 5); add (2.0, 7); add (3.0, 11); add (4.0, 12);
}
{
    let mut sequences: Vec< Sequence> = notes.iter ().map (| note | note.render(44100)).collect ();
let music = merge (&sequences);

let spec = hound::WavSpec {
    channels: 1,
    sample_rate: 44100,
    bits_per_sample: 16
};
let mut writer = hound::WavWriter::create("output.wav", spec).unwrap();
for t in music.samples.iter () {
    writer.write_sample(*t as i16).unwrap();
}
}
{

    let mut sequences: Vec< Sequence> = notes.iter ().map (| note | note.render(format.samples_rate.0 as i32)).collect ();
let music = merge (&sequences);


let mut data_source = music.samples.iter ().map (| sample | *sample as f32);
 for _whatever in 0..0 {
println!( "{}", data_source.next ().unwrap());
}


    // Produce a sinusoid of maximum amplitude.
    //let mut data_source = (0u64..).map(|t| t as f32 * 440.0 * 2.0 * 3.141592 / format.samples_rate.0 as f32)     // 440 Hz
    //                              .map(|t| t.sin());

    loop {
        match channel.append_data(32768) {
            cpal::UnknownTypeBuffer::U16(mut buffer) => {
                for (sample, value) in buffer.chunks_mut(format.channels.len()).zip(&mut data_source) {
                    let value = ((value * 0.5 + 0.5) * std::u16::MAX as f32) as u16;
                    for out in sample.iter_mut() { *out = value; }
                }
            },

            cpal::UnknownTypeBuffer::I16(mut buffer) => {
                for (sample, value) in buffer.chunks_mut(format.channels.len()).zip(&mut data_source) {
                    let value = (value * std::i16::MAX as f32) as i16;
                    for out in sample.iter_mut() { *out = value; }
                }
            },

            cpal::UnknownTypeBuffer::F32(mut buffer) => {
                for (sample, value) in buffer.chunks_mut(format.channels.len()).zip(&mut data_source) {
                    for out in sample.iter_mut() { *out = value; }
                }
            },
        }
        //hack: terminate by dropping one sample each time.
        if let None = data_source.next () {break;}

        channel.play();
    }
}
}

//extern crate fluidsynth;
////extern crate time;

//use fluidsynth::*;
//use std::thread;

//fn main() {
    //let mut settings = settings::Settings::new();
    //let mut syn = synth::Synth::new(&mut settings);
    //let _adriver = audio::AudioDriver::new(&mut settings, &mut syn);
    //syn.sfload("/usr/share/sounds/sf2/FluidR3_GM.sf2", 1);

    //for x in 0..12 {
        //let num: i32 = x;
        //let key = 60 + num;
        //syn.noteon(0, key, 80);
        //thread::sleep_ms(1000);
        //syn.noteoff(0, key);
    //}
//}

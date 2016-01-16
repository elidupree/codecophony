
extern crate cpal;


use std::cmp::{min, max};

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


fn main() {
    let endpoint = cpal::get_default_endpoint().unwrap();
    let format = endpoint.get_supported_formats_list().unwrap().next().unwrap();
    let mut channel = cpal::Voice::new(&endpoint, &format).unwrap();
    
    println!( "sample rate is {}", format.samples_rate.0);
let mut sequences: Vec< Sequence> = Vec::new ();
{
let mut add = | time: f64, pitch | {sequences.push (
	hack_note {start: time/4.0, frequency: 440.0*semitone_ratio.powi (pitch), amplitude: 4000.0,}.render (format.samples_rate.0 as i32))};
add (0.0, 0); add (1.5, 5); add (2.0, 7); add (3.0, 11); add (4.0, 12);
}
let music = merge (&sequences);
let mut data_source = music.samples.iter ().cycle ().map (| sample | *sample as f32);
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

        channel.play();
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

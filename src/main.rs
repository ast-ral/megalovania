use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use cpal::{StreamData, UnknownTypeOutputBuffer as UTOB};
use std::f64::consts::PI;
use std::thread;
use std::sync::mpsc::channel;

fn main() {
	let host = cpal::default_host();
	let event_loop = host.event_loop();
	let device = host.default_output_device().expect("no output device found");

	let mut supported_formats_range = device
		.supported_output_formats()
		.expect("error while querying formats");
	let format = supported_formats_range.next()
		.expect("no format supported")
		.with_max_sample_rate();

	let sample_rate = format.sample_rate.0;
	let channel_count = usize::from(format.channels);

	let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

	event_loop.play_stream(stream_id).expect("failed to play stream");

	let mut counter: u64 = 0;

	let mut source = opening_measures();

	let (tx, rx) = channel();

	thread::spawn(move || {
		let mut terminating = false;

		event_loop.run(move |stream_id, stream_result| {
			if terminating {
				tx.send(()).expect("thread sending error");
				thread::park();
			}

			let stream_data = match stream_result {
				Ok(data) => data,
				Err(error) => panic!(format!(
					"an error occured on stream {:?}: {}",
					stream_id,
					error,
				)),
			};

			let static_data = StaticData {
				counter: &mut counter,
				sample_rate,
				channel_count,
				source: &mut source,
				terminating: &mut terminating,
			};

			if let StreamData::Output {buffer: buffer_enum} = stream_data {
				match buffer_enum {
					UTOB::U16(mut buffer) => {
						fill_buffer(
							static_data,
							&mut *buffer,
							|f| {
								((1.0 + f) * f64::from(u16::MAX / 2)) as u16
							},
						)
					},
					UTOB::I16(mut buffer) => {
						fill_buffer(
							static_data,
							&mut *buffer,
							|f| {
								(f * f64::from(i16::MAX)) as i16
							},
						)
					},
					UTOB::F32(mut buffer) => {
						fill_buffer(
							static_data,
							&mut *buffer,
							|f| f as f32,
						)
					},
				}
			}
		});
	});

	rx.recv().expect("thread reception error");
}

struct StaticData<'a> {
	counter: &'a mut u64,
	sample_rate: u32,
	channel_count: usize,
	source: &'a mut Source,
	terminating: &'a mut bool,
}

const VOLUME: f64 = 0.1;

fn fill_buffer<'a, T, F: Fn(f64) -> T>(
	static_data: StaticData<'a>,
	buffer: &'a mut [T],
	closure: F,
) {
	let StaticData {
		counter,
		sample_rate,
		channel_count,
		source,
		terminating,
	} = static_data;

	assert!(buffer.len() % channel_count == 0);

	for i in 0 .. (buffer.len() / channel_count) {
		let t = (*counter as f64) / (sample_rate as f64);
		let val = match play_source(t, source) {
			Some(signal) => signal * VOLUME,
			None => {
				*terminating = true;
				0.0
			},
		};

		for j in 0 .. channel_count {
			buffer[channel_count * i + j] = closure(val);
		}

		*counter += 1;
	}
}

const A4: f64 = 440.0;
const TAU: f64 = 2.0 * PI;
const BPM: f64 = 120.0;

enum Instruction {
	Note {pitch: i32, length: f64},
	Rest {length: f64},
}

impl Instruction {
	fn length(&self) -> f64 {
		match self {
			Instruction::Note {length, ..} => *length,
			Instruction::Rest {length} => *length,
		}
	}
}

struct Source {
	instructions: Vec<Instruction>,
	start_of_instruction: f64,
	current_instruction: usize,
}

impl Source {
	fn new(instructions: Vec<Instruction>) -> Self {
		Source {
			instructions,
			start_of_instruction: 0.0,
			current_instruction: 0,
		}
	}
}

// this returns None to signal end of source
fn play_source(t: f64, source: &mut Source) -> Option<f64> {
	let instructions = &source.instructions;
	let start_of_instruction = &mut source.start_of_instruction;
	let current_instruction = &mut source.current_instruction;

	let measure_time = 60.0 / BPM * 4.0;

	if *current_instruction >= instructions.len() {
		return None;
	}

	let current_length = instructions[*current_instruction].length() * measure_time;

	if t > *start_of_instruction + current_length {
		*start_of_instruction += current_length;
		*current_instruction += 1;

		if *current_instruction >= instructions.len() {
			return None;
		}
	}

	Some(match instructions[*current_instruction] {
		Instruction::Note {pitch, length} => {
			note_gen(t - *start_of_instruction, pitch, length * measure_time)
		},
		Instruction::Rest {..} => 0.0,
	})
}

fn note_gen(t: f64, pitch: i32, length: f64) -> f64 {
	(TAU * t * pitch_compute(pitch)).sin() * envelope(t / length)
}

fn pitch_compute(pitch: i32) -> f64 {
	A4 * 2.0f64.powf(1.0 / 12.0).powi(pitch)
}

fn envelope(x: f64) -> f64 {
	if x < 0.0 || x > 1.0 {
		return 0.0;
	}

	if x < 0.1 {
		return x * 10.0;
	}

	if x > 0.9 {
		return (1.0 - x) * 10.0;
	}

	return 1.0;
}

const L8TH: f64 = 1.0 / 8.0;
const L16TH: f64 = 1.0 / 16.0;

fn opening_measures() -> Source {
	use Instruction::{Note, Rest};

	Source::new(vec![
		Note {pitch: -7, length: L16TH},
		Note {pitch: -7, length: L16TH},
		Note {pitch: 5, length: L16TH},
		Rest {length: L16TH},
		Note {pitch: 0, length: L8TH},
		Rest {length: L16TH},
		Note {pitch: -1, length: L8TH},
		Note {pitch: -2, length: L8TH},
		Note {pitch: -4, length: L8TH},
		Note {pitch: -7, length: L16TH},
		Note {pitch: -4, length: L16TH},
		Note {pitch: -2, length: L16TH},

		Note {pitch: -9, length: L16TH},
		Note {pitch: -9, length: L16TH},
		Note {pitch: 5, length: L16TH},
		Rest {length: L16TH},
		Note {pitch: 0, length: L8TH},
		Rest {length: L16TH},
		Note {pitch: -1, length: L8TH},
		Note {pitch: -2, length: L8TH},
		Note {pitch: -4, length: L8TH},
		Note {pitch: -7, length: L16TH},
		Note {pitch: -4, length: L16TH},
		Note {pitch: -2, length: L16TH},

		Note {pitch: -10, length: L16TH},
		Note {pitch: -10, length: L16TH},
		Note {pitch: 5, length: L16TH},
		Rest {length: L16TH},
		Note {pitch: 0, length: L8TH},
		Rest {length: L16TH},
		Note {pitch: -1, length: L8TH},
		Note {pitch: -2, length: L8TH},
		Note {pitch: -4, length: L8TH},
		Note {pitch: -7, length: L16TH},
		Note {pitch: -4, length: L16TH},
		Note {pitch: -2, length: L16TH},

		Note {pitch: -11, length: L16TH},
		Note {pitch: -11, length: L16TH},
		Note {pitch: 5, length: L16TH},
		Rest {length: L16TH},
		Note {pitch: 0, length: L8TH},
		Rest {length: L16TH},
		Note {pitch: -1, length: L8TH},
		Note {pitch: -2, length: L8TH},
		Note {pitch: -4, length: L8TH},
		Note {pitch: -7, length: L16TH},
		Note {pitch: -4, length: L16TH},
		Note {pitch: -2, length: L16TH},

		Rest {length: L16TH},
	])
}

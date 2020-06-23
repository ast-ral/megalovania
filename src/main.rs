use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use cpal::{StreamData, UnknownTypeOutputBuffer as UTOB};
use std::f32::consts::PI;

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

	let sample_rate = format.sample_rate;
	let channel_count = usize::from(format.channels);

	let multiplier = 2.0 * PI / (sample_rate.0 as f32);

	let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

	event_loop.play_stream(stream_id).expect("failed to play stream");

	let mut counter = 0.0;

	event_loop.run(move |stream_id, stream_result| {
		let stream_data = match stream_result {
			Ok(data) => data,
			Err(error) => panic!(format!(
				"an error occured on stream {:?}: {}",
				stream_id,
				error,
			))
		};

		if let StreamData::Output {buffer: buffer_enum} = stream_data {
			match buffer_enum {
				/*
				UTOB::U16(mut buffer) => {
					panic!("u16 format");
					for i in 0 .. buffer.len() {
						buffer[i] = u16::MAX / 2;
					}
				},
				UTOB::I16(mut buffer) => {
					panic!("i16 format");
					for i in 0 .. buffer.len() {
						buffer[i] = 0;
					}
				},
				*/
				UTOB::F32(mut buffer) => {
					assert!(buffer.len() % channel_count == 0);
					let hz = 440.0;

					for i in 0 .. (buffer.len() / channel_count) {
						let val = (hz * multiplier * counter).sin() * 0.01;

						for j in 0 .. channel_count {
							buffer[channel_count * i + j] = val;
						}

						counter += 1.0;
					}
				},
				_ => panic!("not f32?"),
			}
		}
	});
}

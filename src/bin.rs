use litcontainers::*;
use litaudio::*;
use std::path::{PathBuf};
use littempo::TempoExtractionSettings;
use clap::{App, Arg};

fn main() {
	let (audio_path, settings) = arguments();
	let audio: AudioDeinterleavedC<f64, U1, Dynamic> = litaudioio::read_audio(&audio_path).unwrap();

	let tempo_sections = littempo::extract_tempo(&audio, &settings);

	println!("Found tempo sections:");
	for s in &tempo_sections {
		println!("{:#?}", s);
	}
}

fn arguments() -> (PathBuf, TempoExtractionSettings) {
	let matches = App::new("LitTempo Extraction")
		.version("1.0")
		.author("Egor Dmitriev <egordmitriev2@gmail.com>")
		.about("Tool to extract tempo information from audio files")
		.arg(Arg::with_name("input")
			.help("Input audio file")
			.required(true))
		.arg(Arg::with_name("output")
			.short("o")
			.help("Output directory")
			.takes_value(true))
		.arg(Arg::with_name("visualize")
			.short("v")
			.help("Creates plots with calculated data"))
		.arg(Arg::with_name("click")
			.short("c")
			.help("Creates audio file with click track"))
		.get_matches();
	let default_settings = littempo::TempoExtractionSettingsBuilder::default().build().unwrap();

	let path = PathBuf::from(matches.value_of("input").unwrap());
	assert!(path.exists(), "Input file doesnt exist!");

	let filename = (&path).file_stem().unwrap().to_str().unwrap().to_string();
	let out_path = matches.value_of("output").map(|s| PathBuf::from(s))
		.unwrap_or(default_settings.save_path().clone())
		.join(filename);

	if !out_path.exists() {
		std::fs::create_dir_all(&out_path).unwrap();
	}

	let settings_builder = littempo::TempoExtractionSettingsBuilder::default()
		.save_click_track(matches.is_present("click"))
		.visualize(matches.is_present("visualize"))
		.save_path(out_path)
		.verbose(true)
		.build().unwrap();

	(path, settings_builder)
}
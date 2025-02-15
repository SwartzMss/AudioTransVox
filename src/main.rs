use clap::{Parser, Subcommand};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
mod audio_capture;
use audio_capture::AudioCapture;
mod download_model;
use download_model::download_file;
use std::path::Path;

mod audio_transcribe;
use audio_transcribe::Whisper;

mod translate;

#[derive(Parser)]
#[command(name = "AudioTransVox", version = "1.0", author = "Swartz Lubel <swartz_luel@outlook.com>", about = "Audio translation tool", long_about = "AudioTransVox is a tool for capturing, transcribing, and translating audio files.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn ensure_model_exists(model_path: &str, download_url: &str) {
    if !Path::new(model_path).exists() {
        println!("Model file not found at {}. Downloading...", model_path);
        download_file(download_url, model_path);
    } 
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Capture audio from the default output", long_about = "Capture audio from the default output and save it to a file with a timestamped name.\n\nUsage:\n  audio_trans_vox.exe capture")]
    Capture,
    #[command(about = "Transcribe audio to text", long_about = "Transcribe the given audio file to text and display the result in the terminal.\n\nArguments:\n  -i, --input <FILE>    The input audio file to transcribe\n  -o, --output <FILE>   The output text file to save the transcription result\n\nUsage:\n  audio_trans_vox.exe transcribe -i <FILE> [-o <FILE>]")]
    Transcribe {
        #[arg(short, long, value_name = "FILE", help = "The input audio file to transcribe")]
        input: String,
        #[arg(short, long, value_name = "FILE", help = "The output text file to save the transcription result")]
        output: Option<String>,
    },
    #[command(about = "Translate text to Chinese", long_about = "Translate the given text file to Chinese and display the result in the terminal.\n\nArguments:\n  -i, --input <FILE>    The input text file to translate\n\nUsage:\n  audio_trans_vox.exe translate -i <FILE>")]
    Translate {
        #[arg(short = 'i', long = "input", value_name = "FILE", help = "The input text file to translate")]
        input: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Capture => {
            let output = format!("audio_{}.wav", chrono::Local::now().format("%Y%m%d%H%M%S"));
            println!("Capturing audio to {}", output);

            let mut audio_capture = AudioCapture::new(output);
            audio_capture.start();
            println!("Audio capture started. Press Ctrl+C to stop.");
            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();
            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            }).expect("Error setting Ctrl-C handler");

            while running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            audio_capture.stop();
            println!("Audio capture stopped.");
        }
        Commands::Transcribe { input, output } => {
            let model_path = "models/ggml-base.bin";
            let download_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
            ensure_model_exists(model_path, download_url);
            println!("Transcribing audio file {}", input);
            let mut whisper = Whisper::new("models/ggml-base.bin");
            let result = whisper
               .transcribe_file(input)
               .expect("Transcription failed");
            println!("Transcription result:\n{}", result);

            if let Some(output_file) = output {
                std::fs::write(output_file, &result).expect("Failed to write to output file");
                println!("Transcription result saved to {}", output_file);
            }
        }
        Commands::Translate { input } => {
            println!("Translating text file {} to Chinese", input);
            let content = std::fs::read_to_string(&input).expect("Failed to read input file");
            let model_path = "models/model.safetensors";
            let download_url = "https://huggingface.co/Helsinki-NLP/opus-mt-en-zh/resolve/refs%2Fpr%2F26/model.safetensors";
            ensure_model_exists(model_path, download_url);

            let tokenizer_path_en = "models/tokenizer-marian-base-en.json";
            let tokenizer_path_zh = "models/tokenizer-marian-base-zh.json";

            let mut translator = translate::Translator::new(model_path,tokenizer_path_en,tokenizer_path_zh).expect("Failed to load translator model");
            let result = translator.translate(&content).expect("Translation failed");
            println!("Translation result:\n{}", result);
        }
    }
}
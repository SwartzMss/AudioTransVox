use clap::{Parser, Subcommand};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
mod audio_capture;
use audio_capture::AudioCapture;

#[derive(Parser)]
#[command(name = "AudioTransVox", version = "1.0", author = "Swartz Lubel <swartz_luel@outlook.com>", about = "Audio translation tool", long_about = "AudioTransVox is a tool for capturing, transcribing, and translating audio files.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Capture audio from the default output", long_about = "Capture audio from the default output and save it to a file with a timestamped name.\n\nUsage:\n  audio_trans_vox.exe capture")]
    Capture,
    #[command(about = "Transcribe audio to text", long_about = "Transcribe the given audio file to text and display the result in the terminal.\n\nArguments:\n  -i, --input <FILE>    The input audio file to transcribe\n\nUsage:\n  audio_trans_vox.exe transcribe -i <FILE>")]
    Transcribe {
        #[arg(short, long, value_name = "FILE", help = "The input audio file to transcribe")]
        input: String,
    },
    #[command(about = "Translate text to another language", long_about = "Translate the given text file to the specified language and save the result to an output file.\n\nArguments:\n  -i, --input <FILE>      The input text file to translate\n  -l, --language <LANGUAGE>  The target language for translation\n\nUsage:\n  audio_trans_vox.exe translate -i <FILE> -l <LANGUAGE>")]
    Translate {
        #[arg(short = 'i', long = "input", value_name = "FILE", help = "The input text file to translate")]
        input: String,
        #[arg(short = 'l', long = "language", value_name = "LANGUAGE", help = "The target language for translation")]
        language: String,
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
        Commands::Transcribe { input } => {
            println!("Transcribing audio file {}", input);
            // Add your audio transcription logic here
        }
        Commands::Translate { input, language } => {
            println!("Translating audio file {} in {}", input, language);
            // Add your text translation logic here
        }
    }
}
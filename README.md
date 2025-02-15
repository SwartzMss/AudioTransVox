# AudioTransVox

AudioTransVox 是一个音频录制、转写与翻译工具，基于 [Rust](https://www.rust-lang.org/) 开发。它包含以下核心功能：

1. **音频采集**：将当前播放的音频捕获并保存为单声道的 WAV 文件。  
2. **语音转写**：使用 [Whisper](https://github.com/openai/whisper)（通过 [whisper-rs](https://github.com/tazz4843/whisper-rs)）对音频文件进行转写，自动检测音频语言，输出识别结果。  
3. **文本翻译**：使用 [Marian](https://huggingface.co/Helsinki-NLP) 模型（通过 [candle-transformers](https://github.com/huggingface/candle/tree/main/candle-transformers)）对文本从英语翻译到中文。

## 功能概览

本项目内包含以下主要模块：
- `audio_capture.rs`：提供捕获系统音频输出、混合至单声道并写入 16-bit PCM WAV 文件的功能。  
- `audio_transcribe.rs`：封装了 Whisper 模型，对传入的 WAV 文件进行转写并输出文本。  
- `translate.rs`：基于 Marian 的翻译功能，自动判断是否为英文文本，如果是则翻译，否则原样返回。  
- `download_model.rs`：封装了模型下载逻辑，若本地未检测到指定的模型文件，则会从指定 URL 自动下载并存储到本地。

## 安装 & 依赖

1. **Rust 环境**  
   - 请确保已安装 Rust（推荐使用 [rustup](https://www.rust-lang.org/tools/install)），并保证 `cargo` 命令可用。  

2. **依赖库**  
   - 本项目中使用了以下主要依赖：
     - [cpal](https://github.com/RustAudio/cpal) 用于音频输入/输出捕获。
     - [hound](https://github.com/ruuda/hound) 读写 WAV 文件。
     - [samplerate](https://github.com/WebAudio/cpal) 用于音频重采样。
     - [whisper-rs](https://github.com/tazz4843/whisper-rs) Whisper 语音识别。
     - [candle-transformers](https://github.com/huggingface/candle/tree/main/candle-transformers) 和相关 Candle 库，用于 Marian 模型翻译。
     - [tokenizers](https://github.com/huggingface/tokenizers) 用于分词。
     - [reqwest](https://github.com/seanmonstar/reqwest) 用于网络请求（下载模型）。
     - [clap](https://github.com/clap-rs/clap) 命令行参数解析。
   - 在执行 `cargo build` 时，cargo 会自动下载并编译所需依赖。

3. **Whisper 模型文件**  
   - 默认会在 `models` 目录下查找 `ggml-base.bin`，若不存在则会自动从 Hugging Face 下载。
   - 若需使用其他 Whisper 模型文件，可修改 `main.rs` 中 `Transcribe` 命令的 `model_path` 与对应的下载地址 `download_url`。

4. **Marian 翻译模型文件**  
   - 默认会在 `models` 目录下查找 `model.safetensors` (以及对应分词器 `tokenizer-marian-base-en.json`、`tokenizer-marian-base-zh.json`)。  
   - 不存在时会从 Hugging Face 下载一份示例模型文件并存储到 `models` 文件夹下。  
   - 若想替换成其他支持英->中翻译的 Marian 模型，可在代码中调整相关配置。

## 编译

1. **克隆或下载本项目**  
   ```bash
   git clone https://github.com/YourName/AudioTransVox.git
   cd AudioTransVox
   ```

2. **编译**  
   ```bash
   cargo build --release
   ```

## 运行

### 捕获音频（默认为系统输出设备）

```bash
cargo run --release -- capture
```

执行此命令后，程序将开始录制当前系统的音频输出，并保存为形如 `audio_20250101123000.wav` 的文件。按 Ctrl+C 停止录制，并写回 WAV 头信息。

### 转写音频

```bash
cargo run --release -- transcribe -i your_audio.wav [-o output.txt]
```

- `-i <FILE>`：指定输入的 WAV 文件
- `-o <FILE>`：（可选）指定输出文本文件路径；如不提供则只在终端打印结果

### 翻译文本

```bash
cargo run --release -- translate -i your_text.txt
```

- `-i <FILE>`：指定需要翻译的文本文件
- 若文本主要为英文，则会自动翻译成中文并打印在终端；如果是非英文文本，则原样返回。

## 遗留问题

### Debug 模式下的编译问题

在 debug 模式下编译时，可能会遇到动态库和静态库编译冲突的问题。
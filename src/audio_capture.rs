use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};
use num_traits::ToPrimitive;

pub struct AudioCapture {
    stream: Option<Stream>,
    file_name: String,
    file: Option<Arc<Mutex<File>>>,
}

impl AudioCapture {
    pub fn new(file_name: String) -> Self {
        Self {
            stream: None,
            file_name,
            file: None,
        }
    }

    pub fn start(&mut self) {
        let host = cpal::default_host();
        // 这里依然使用输出设备来捕获系统输出（注意需操作系统支持 loopback 模式）
        let device = host.default_output_device().expect("Failed to get default output device");
        println!("Using output device: {}", device.name().unwrap_or("Unknown".to_string()));

        let config = device.default_output_config().expect("Failed to get default output config");
        println!("Default output config: {:?}", config);

        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        // 创建输出文件，并写入 WAV 文件头的占位数据
        let file = Arc::new(Mutex::new(File::create(&self.file_name).expect("Failed to create output file")));
        {
            let mut file_lock = file.lock().unwrap();
            write_wav_header(&mut *file_lock, &config, sample_format);
        }
        // 保存文件句柄，方便后续更新文件头
        self.file = Some(file.clone());

        let err_fn = |err| eprintln!("An error occurred on the output audio stream: {}", err);

        let stream = match sample_format {
            SampleFormat::I8 => self.capture::<i8>(&device, &config, file.clone(), err_fn),
            SampleFormat::I16 => self.capture::<i16>(&device, &config, file.clone(), err_fn),
            SampleFormat::I32 => self.capture::<i32>(&device, &config, file.clone(), err_fn),
            SampleFormat::I64 => self.capture::<i64>(&device, &config, file.clone(), err_fn),
            SampleFormat::U8 => self.capture::<u8>(&device, &config, file.clone(), err_fn),
            SampleFormat::U16 => self.capture::<u16>(&device, &config, file.clone(), err_fn),
            SampleFormat::U32 => self.capture::<u32>(&device, &config, file.clone(), err_fn),
            SampleFormat::U64 => self.capture::<u64>(&device, &config, file.clone(), err_fn),
            SampleFormat::F32 => self.capture::<f32>(&device, &config, file.clone(), err_fn),
            SampleFormat::F64 => self.capture::<f64>(&device, &config, file.clone(), err_fn),
            _ => panic!("Unsupported sample format"),
        };

        let stream = stream.expect("Failed to build input stream");
        stream.play().expect("Failed to play the stream");
        self.stream = Some(stream);
    }

    fn capture<T>(
        &self,
        device: &cpal::Device,
        config: &StreamConfig,
        file: Arc<Mutex<File>>,
        err_fn: fn(cpal::StreamError),
    ) -> Result<Stream, cpal::BuildStreamError>
    where
        T: cpal::Sample + cpal::SizedSample + ToPrimitive,
    {
        let channels = config.channels as usize;

        device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut file_lock = file.lock().unwrap();
                println!("Captured {} frames", data.len() / channels);
                for frame in data.chunks(channels) {
                    for &sample in frame {
                        // 根据采样类型大小写入对应的字节数据
                        if std::mem::size_of::<T>() == 1 {
                            let byte = sample.to_u8().unwrap();
                            file_lock.write_all(&[byte]).unwrap();
                        } else if std::mem::size_of::<T>() == 2 {
                            let bytes = sample.to_u16().unwrap().to_le_bytes();
                            file_lock.write_all(&bytes).unwrap();
                        } else if std::mem::size_of::<T>() == 4 {
                            let bytes = sample.to_f32().unwrap().to_le_bytes();
                            file_lock.write_all(&bytes).unwrap();
                        } else if std::mem::size_of::<T>() == 8 {
                            if let Some(val) = sample.to_u64() {
                                let bytes = val.to_le_bytes();
                                file_lock.write_all(&bytes).unwrap();
                            } else {
                                let bytes = sample.to_f64().unwrap().to_le_bytes();
                                file_lock.write_all(&bytes).unwrap();
                            }
                        } else {
                            panic!("Unsupported sample type size");
                        }
                    }
                }
            },
            err_fn,
            None,
        )
    }

    /// 停止捕获，暂停流并更新 WAV 文件头信息
    pub fn stop(&mut self) {
        if let Some(stream) = &self.stream {
            stream.pause().expect("Failed to pause stream");
        }
        if let Some(file_arc) = &self.file {
            let mut file = file_arc.lock().unwrap();
            update_wav_header(&mut *file);
            println!("WAV header updated.");
        }
    }
}

fn write_wav_header(file: &mut File, config: &StreamConfig, sample_format: SampleFormat) {
    let channels = config.channels as u16;
    let sample_rate = config.sample_rate.0;
    let bits_per_sample = match sample_format {
        SampleFormat::I8 | SampleFormat::U8 => 8,
        SampleFormat::I16 | SampleFormat::U16 => 16,
        SampleFormat::I32 | SampleFormat::U32 | SampleFormat::F32 => 32,
        SampleFormat::I64 | SampleFormat::U64 | SampleFormat::F64 => 64,
        _ => panic!("Unsupported sample format"),
    };

    // 如果采样格式为浮点类型，则应设置 AudioFormat 为 3 (IEEE Float)
    let audio_format: u16 = match sample_format {
        SampleFormat::F32 | SampleFormat::F64 => 3,
        _ => 1,
    };

    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8) as u16;

    let mut header = vec![
        b'R', b'I', b'F', b'F', // ChunkID
        0, 0, 0, 0,             // ChunkSize (占位)
        b'W', b'A', b'V', b'E', // Format
        b'f', b'm', b't', b' ', // Subchunk1ID
        16, 0, 0, 0,            // Subchunk1Size (16 for PCM / IEEE float)
    ];
    // 写入 AudioFormat（根据音频格式）
    header.extend_from_slice(&audio_format.to_le_bytes());
    // 写入 NumChannels
    header.push(channels as u8);
    header.push((channels >> 8) as u8);
    // 写入 SampleRate
    header.push((sample_rate & 0xFF) as u8);
    header.push(((sample_rate >> 8) & 0xFF) as u8);
    header.push(((sample_rate >> 16) & 0xFF) as u8);
    header.push(((sample_rate >> 24) & 0xFF) as u8);
    // 写入 ByteRate
    header.push((byte_rate & 0xFF) as u8);
    header.push(((byte_rate >> 8) & 0xFF) as u8);
    header.push(((byte_rate >> 16) & 0xFF) as u8);
    header.push(((byte_rate >> 24) & 0xFF) as u8);
    // 写入 BlockAlign
    header.push(block_align as u8);
    header.push((block_align >> 8) as u8);
    // 写入 BitsPerSample
    header.push(bits_per_sample as u8);
    header.push((bits_per_sample >> 8) as u8);
    // 写入 "data" chunk ID 和占位的 Subchunk2Size
    header.extend_from_slice(&[b'd', b'a', b't', b'a']);
    header.extend_from_slice(&[0, 0, 0, 0]); // Subchunk2Size (占位)

    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&header).unwrap();
}


/// 在捕获结束时更新 WAV 文件头中的文件大小和数据块大小字段
fn update_wav_header(file: &mut File) {
    let file_size = file.seek(SeekFrom::End(0)).unwrap();
    let data_chunk_size = file_size - 44;
    file.seek(SeekFrom::Start(4)).unwrap();
    file.write_all(&(file_size - 8).to_le_bytes()).unwrap();
    file.seek(SeekFrom::Start(40)).unwrap();
    // 这里假设数据块大小为 u32 类型
    file.write_all(&(data_chunk_size as u32).to_le_bytes()).unwrap();
}

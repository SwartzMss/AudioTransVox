use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::{Write, Seek, SeekFrom};
use num_traits::ToPrimitive;
use std::time::Duration;
use std::thread;
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
        let device = host
            .default_output_device()
            .expect("Failed to get default output device");
        println!(
            "Using output device: {}",
            device.name().unwrap_or("Unknown".to_string())
        );

        let config = device
            .default_output_config()
            .expect("Failed to get default output config");
        println!("Default output config: {:?}", config);

        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        // 创建输出文件，并写入 WAV 文件头的占位数据
        let file = Arc::new(Mutex::new(
            File::create(&self.file_name).expect("Failed to create output file"),
        ));
        {
            let mut file_lock = file.lock().unwrap();
            // 调用时去掉 sample_format 参数，因为我们固定输出为 16-bit PCM 单声道
            write_wav_header(&mut *file_lock, &config);
        }
        // 保存文件句柄，方便后续更新文件头
        self.file = Some(file.clone());

        let err_fn = |err| eprintln!("An error occurred on the output audio stream: {}", err);

        // 只处理 I16, F32, F64 格式，其他格式不支持
        let stream = match sample_format {
            SampleFormat::I16 => self.capture::<i16>(&device, &config, file.clone(), err_fn),
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

                // 判断捕获到的是单声道还是立体声
                if channels == 1 {
                    // 单声道：直接写入每个采样
                    for &sample in data {
                        Self::write_sample(&mut file_lock, sample);
                    }
                } else if channels == 2 {
                    // 立体声：混合左右通道（均值）转换为单声道后写入
                    for frame in data.chunks(2) {
                        let mut left_sample = frame[0].to_f32().unwrap();
                        let mut right_sample = frame[1].to_f32().unwrap();
                    
                        // 对i16样本进行归一化
                        if std::mem::size_of::<T>() == 2 {
                            left_sample /= 32768.0;
                            right_sample /= 32768.0;
                        }
                    
                        let mixed_sample = (left_sample + right_sample) / 2.0;
                        let pcm_value = (mixed_sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        Self::write_sample(&mut file_lock, pcm_value);
                    }
                } else {
                    panic!("Unsupported number of channels: {}", channels);
                }
            },
            err_fn,
            None,
        )
    }

    /// 将采样数据写入文件，转换为 16-bit PCM 格式
    fn write_sample<T>(file_lock: &mut File, sample: T)
    where
        T: cpal::Sample + cpal::SizedSample + ToPrimitive,
    {
        if std::mem::size_of::<T>() == 4 {
            // 对于 F32 和 F64，将浮点数转换为 16-bit PCM
            let pcm_value = (sample.to_f32().unwrap() * 32767.0)
                .clamp(-32768.0, 32767.0) as i16;
            let bytes = pcm_value.to_le_bytes();
            file_lock.write_all(&bytes).unwrap();
        } else if std::mem::size_of::<T>() == 2 {
            // 对于 I16，直接写入
            let bytes = sample.to_i16().unwrap().to_le_bytes();
            file_lock.write_all(&bytes).unwrap();
        } else {
            panic!("Unsupported sample type size");
        }
    }

    pub fn stop(&mut self) {
        // 取出流对象并暂停
        if let Some(stream) = self.stream.take() {
            stream.pause().expect("Failed to pause stream");
            // 短暂等待确保回调完成（根据实际情况调整等待时间）
            thread::sleep(Duration::from_millis(100));
        }

        // 更新 WAV 文件头前先 flush 文件，确保所有数据已写入磁盘
        if let Some(file_arc) = &self.file {
            let mut file = file_arc.lock().unwrap();
            file.flush().expect("Failed to flush file");
            update_wav_header(&mut *file);
            println!("WAV header updated.");
        }
    }
}

/// 写入 WAV 文件头  
/// 固定输出为 16-bit PCM 格式，并且如果设备为立体声则混合为单声道输出，
fn write_wav_header(file: &mut File, config: &StreamConfig) {
    // 如果输入是立体声，则输出为单声道（1 通道）
    let header_channels: u16 = if config.channels == 2 {
        1
    } else {
        config.channels as u16
    };
    let sample_rate = config.sample_rate.0;
    let bits_per_sample = 16; // 固定为 16-bit PCM
    let audio_format: u16 = 1; // PCM 格式
    let byte_rate = sample_rate * header_channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = header_channels * (bits_per_sample / 8) as u16;

    let mut header = vec![
        b'R', b'I', b'F', b'F', // ChunkID
        0, 0, 0, 0,             // ChunkSize (占位)
        b'W', b'A', b'V', b'E', // Format
        b'f', b'm', b't', b' ', // Subchunk1ID
        16, 0, 0, 0,            // Subchunk1Size (16 for PCM)
    ];

    // 写入 AudioFormat（PCM 格式）
    header.extend_from_slice(&audio_format.to_le_bytes());
    // 写入通道数（这里固定为 header_channels，即单声道时为 1）
    header.push(header_channels as u8);
    header.push((header_channels >> 8) as u8);
    // 写入采样率
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
    // 写入 "data" chunk ID 及占位的 Subchunk2Size
    header.extend_from_slice(&[b'd', b'a', b't', b'a']);
    header.extend_from_slice(&[0, 0, 0, 0]); // 占位

    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&header).unwrap();
}

/// 更新 WAV 文件头中的文件大小和数据块大小字段
fn update_wav_header(file: &mut File) {
    let file_size = file.seek(SeekFrom::End(0)).unwrap();
    let data_chunk_size = file_size - 44;
    file.seek(SeekFrom::Start(4)).unwrap();
    file.write_all(&(data_chunk_size as u32).to_le_bytes()).unwrap();
    file.seek(SeekFrom::Start(40)).unwrap();
    file.write_all(&(data_chunk_size as u32).to_le_bytes()).unwrap();
}

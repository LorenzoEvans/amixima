use crate::ontology::{EffectNode, Soundcourse};
use color_eyre::Result;
use hound::{WavSpec, WavWriter};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use std::fs::File;
use std::path::Path;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioPlayer {
    _stream: Option<cpal::Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    cursor: Arc<Mutex<usize>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            _stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            cursor: Arc::new(Mutex::new(0)),
        }
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, _sample_rate: u32, channels: u16) -> Result<()> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| color_eyre::eyre::eyre!("no output device"))?;
        let config = device.default_output_config()?;

        let samples = Arc::new(Mutex::new(samples));
        let cursor = Arc::new(Mutex::new(0));

        let samples_cb = Arc::clone(&samples);
        let cursor_cb = Arc::clone(&cursor);

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let s = match samples_cb.lock() { Ok(s) => s, Err(_) => return };
                let mut c = match cursor_cb.lock() { Ok(c) => c, Err(_) => return };
                for frame in data.chunks_mut(channels as usize) {
                    for sample in frame {
                        if *c < s.len() {
                            *sample = s[*c];
                            *c += 1;
                        } else {
                            *sample = 0.0;
                        }
                    }
                }
            },
            |err| eprintln!("audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self._stream = Some(stream);
        self.samples = samples;
        self.cursor = cursor;

        Ok(())
    }

    pub fn stop(&mut self) {
        self._stream = None;
    }

    pub fn is_playing(&self) -> bool {
        if let Ok(c) = self.cursor.lock() {
            if let Ok(s) = self.samples.lock() {
                return *c < s.len();
            }
        }
        false
    }
}

pub struct Soundsculptor;

impl Soundsculptor {
    pub fn apply_soundcourse(
        input_path: &str,
        output_path: &str,
        soundcourse: &Soundcourse,
    ) -> Result<()> {
        let (final_samples, sample_rate, channels) = Self::get_processed_samples(input_path, soundcourse)?;

        // Write back to disk (WAV format)
        let spec = WavSpec {
            channels: channels as u16,
            sample_rate: sample_rate as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec)?;
        for &sample in &final_samples {
            writer.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }

        writer.finalize()?;
        Ok(())
    }

    pub fn get_processed_samples(
        input_path: &str,
        soundcourse: &Soundcourse,
    ) -> Result<(Vec<f32>, u32, u16)> {
        let (mut buf1, sample_rate, channels) = Self::decode_file(input_path)?;
        let mut buf2 = vec![0.0; buf1.len()];
        let mut input_is_buf1 = true;

        for effect in &soundcourse.sequence {
            let (input, output) = if input_is_buf1 {
                (&buf1[..], &mut buf2[..])
            } else {
                (&buf2[..], &mut buf1[..])
            };

            match effect {
                EffectNode::EQ { frequency, gain } => {
                    Self::apply_eq(input, output, *frequency, *gain, sample_rate as f32);
                }
                EffectNode::Reverb { room_size, dry_wet } => {
                    Self::apply_reverb(input, output, *room_size, *dry_wet, sample_rate as f32);
                }
                EffectNode::Delay { delay_ms, feedback } => {
                    Self::apply_delay(input, output, *delay_ms, *feedback, sample_rate as f32);
                }
                EffectNode::Compressor { threshold, ratio } => {
                    Self::apply_compressor(input, output, *threshold, *ratio, sample_rate as f32);
                }
                EffectNode::Gain { gain_db } => {
                    Self::apply_gain(input, output, *gain_db);
                }
            }
            input_is_buf1 = !input_is_buf1;
        }

        let final_samples = if input_is_buf1 { buf1 } else { buf2 };
        Ok((final_samples, sample_rate, channels))
    }

    // ... (rest of methods)

    fn apply_gain(input: &[f32], output: &mut [f32], gain_db: f32) {
        let gain = 10.0f32.powf(gain_db / 20.0);
        for i in 0..input.len() {
            output[i] = input[i] * gain;
        }
    }

    fn decode_file(input_path: &str) -> Result<(Vec<f32>, u32, u16)> {
        let src = File::open(input_path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = Path::new(input_path).extension() {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }
        let probed = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;
        let mut format = probed.format;
        let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| color_eyre::eyre::eyre!("no audio track"))?;
        let track_id = track.id;
        let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

        let mut samples = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != track_id { continue; }
            let decoded = decoder.decode(&packet)?;
            match decoded {
                AudioBufferRef::F32(buf) => {
                    for i in 0..buf.frames() {
                        for c in 0..buf.spec().channels.count() {
                            samples.push(buf.chan(c)[i]);
                        }
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for i in 0..buf.frames() {
                        for c in 0..buf.spec().channels.count() {
                            samples.push(buf.chan(c)[i] as f32 / 32768.0);
                        }
                    }
                }
                AudioBufferRef::S32(buf) => {
                    for i in 0..buf.frames() {
                        for c in 0..buf.spec().channels.count() {
                            samples.push(buf.chan(c)[i] as f32 / 2147483648.0);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok((samples, sample_rate, channels))
    }

    fn apply_eq(input: &[f32], output: &mut [f32], freq: f32, gain_db: f32, sample_rate: f32) {
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / 2.0 * (2.0f32.ln() / 2.0 * 1.0 * w0 / w0.sin()).sinh(); // Q = 1.0
        let a = 10.0f32.powf(gain_db / 40.0);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * w0.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha / a;

        let mut x1 = 0.0;
        let mut x2 = 0.0;
        let mut y1 = 0.0;
        let mut y2 = 0.0;

        for i in 0..input.len() {
            let x0 = input[i];
            let y0 = (b0 / a0) * x0 + (b1 / a0) * x1 + (b2 / a0) * x2 - (a1 / a0) * y1 - (a2 / a0) * y2;

            output[i] = y0;

            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
        }
    }

    fn apply_reverb(input: &[f32], output: &mut [f32], size: f32, mix: f32, sample_rate: f32) {
        let delay_times = [0.0297, 0.0371, 0.0411, 0.0437]; // Seconds
        let mut comb_buf = vec![0.0; input.len()];

        for val in output.iter_mut() {
            *val = 0.0;
        }

        for &dt in &delay_times {
            let delay_samples = (dt * sample_rate) as usize;
            let feedback = 0.7 * size;

            comb_buf.fill(0.0);
            for i in 0..input.len() {
                let delayed_sample = if i >= delay_samples {
                    comb_buf[i - delay_samples]
                } else {
                    0.0
                };
                comb_buf[i] = input[i] + delayed_sample * feedback;
                output[i] += comb_buf[i] * 0.25;
            }
        }

        for i in 0..input.len() {
            output[i] = input[i] * (1.0 - mix) + output[i] * mix;
        }
    }

    fn apply_delay(input: &[f32], output: &mut [f32], delay_ms: f32, feedback: f32, sample_rate: f32) {
        let delay_samples = (delay_ms * sample_rate / 1000.0) as usize;

        for i in 0..input.len() {
            let delayed_sample = if i >= delay_samples {
                output[i - delay_samples]
            } else {
                0.0
            };
            output[i] = input[i] + delayed_sample * feedback;
        }
    }

    fn apply_compressor(
        input: &[f32],
        output: &mut [f32],
        threshold_db: f32,
        ratio: f32,
        sample_rate: f32,
    ) {
        let mut envelope = 0.0;
        let attack_time = 0.005;
        let release_time = 0.050;
        let attack_coeff = (-1.0 / (attack_time * sample_rate)).exp();
        let release_coeff = (-1.0 / (release_time * sample_rate)).exp();

        for i in 0..input.len() {
            let abs_sample = input[i].abs();
            if abs_sample > envelope {
                envelope = attack_coeff * envelope + (1.0 - attack_coeff) * abs_sample;
            } else {
                envelope = release_coeff * envelope + (1.0 - release_coeff) * abs_sample;
            }
            let level_db = if envelope > 1e-6 {
                20.0 * envelope.log10()
            } else {
                -100.0
            };
            let gain = if level_db > threshold_db {
                let reduction_db = (level_db - threshold_db) * (1.0 - 1.0 / ratio);
                10.0f32.powf(-reduction_db / 20.0)
            } else {
                1.0
            };
            output[i] = input[i] * gain;
        }
    }
}

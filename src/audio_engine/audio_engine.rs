use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BuildStreamError, Host, InputCallbackInfo, OutputCallbackInfo, Stream, StreamConfig};
use ringbuf::HeapRb;
use std::rc::Rc;

use crate::io_manager;

#[derive(Debug)]
struct AudioError;

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Audio error occurred")
    }
}
impl std::error::Error for AudioError {}

pub struct AudioContext {
    host: Host,
    pub output_device_index: Option<u32>,
    output_stream: Option<Stream>,
    pub input_device_index: Option<u32>,
    input_stream: Option<Stream>,
    pub sample_rate: u32,
    pub monitor: bool,
}

impl AudioContext {
    pub fn new(sample_rate: u32) -> Self {
        // TODO: implement host selection feature
        let host = cpal::default_host();
        let input_device = host.default_input_device();
        let output_device = host.default_output_device();
        let monitor = false;

        AudioContext {
            host,
            output_device_index,
            output_stream: None,
            input_device_index,
            input_stream: None,
            sample_rate,
            monitor,
        }
    }

    pub fn start(&self) {
        self.input_stream.as_ref().unwrap().play().unwrap();
        self.output_stream.as_ref().unwrap().play().unwrap();
    }

    pub fn stop(&self) {
        self.input_stream.as_ref().unwrap().pause().unwrap();
        self.output_stream.as_ref().unwrap().pause().unwrap();
    }

    pub fn toggle_monitor(&mut self) {
        if self.monitor {
            self.stop();
        } else {
            self.start();
        }
    }

    pub fn get_avail_out_devices(&self) -> Vec<String> {
        self.host
            .output_devices()
            .unwrap()
            .map(|device| device.name().unwrap())
            .collect()
    }

    pub fn get_avail_in_devices(&self) -> Vec<String> {
        self.host
            .input_devices()
            .unwrap()
            .map(|device| device.name().unwrap())
            .collect()
    }

    pub fn get_sample_rates(&self) -> [u32; 13] {
        [
            5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200, 96000, 176400,
            192000,
        ]
    }

    pub fn set_output_device(&mut self, device_index: usize) {
        self.output_device_index = device_index;
    }

    pub fn set_input_device(&mut self, device_index: usize) {
        self.input_device_index = device_index;
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    pub fn init_streams(&mut self) -> Result<(), BuildStreamError> {
        // Create StreamConfigs. Input and output configs must currently be the same.
        // in config
        let input_config = self
            .input_device
            .as_ref()
            .map(|_d| StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(self.sample_rate),
                buffer_size: cpal::BufferSize::Default,
            })
            .unwrap();

        // out config
        let output_config = self
            .output_device
            .as_ref()
            .map(|_d| StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(self.sample_rate),
                buffer_size: cpal::BufferSize::Default,
            })
            .unwrap();

        // Create ring buffer for streams to read and write samples to
        let ring_buffer = HeapRb::<f32>::new(4); // 4 is BufferSize::Default
        let (mut producer, mut consumer) = ring_buffer.split();

        // Define stream data handling functions
        let in_data_fn = move |data: &[f32], info: &InputCallbackInfo| {
            for &sample in data {
                producer.push(sample);
            }
        };

        let out_data_fn = move |data: &mut [f32], info: &OutputCallbackInfo| {
            for sample in data {
                *sample = match consumer.pop() {
                    Some(s) => s,
                    None => 0.0,
                }
            }
        };

        // Create Streams.
        // in stream
        let input_stream = self.input_device.as_ref().unwrap().build_input_stream(
            &input_config,
            in_data_fn,
            |_err| panic!("Could not initialize input stream"),
            None,
        )?;

        let output_stream = self.output_device.as_ref().unwrap().build_output_stream(
            &output_config,
            out_data_fn,
            |_err| panic!("Could not initialize output stream"),
            None,
        )?;

        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        Ok(())
    }
}

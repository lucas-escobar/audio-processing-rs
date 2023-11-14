use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, OutputCallbackInfo, StreamConfig};
use ringbuf::{Consumer, HeapRb, Producer, SharedRb};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

type ConsumerT = Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>;
type ProducerT = Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>;

enum RingBufferRole {
    Producer(Arc<Mutex<ProducerT>>),
    Consumer(Arc<Mutex<ConsumerT>>),
}

impl Clone for RingBufferRole {
    fn clone(&self) -> Self {
        match self {
            RingBufferRole::Producer(p) => RingBufferRole::Producer(p.clone()),
            RingBufferRole::Consumer(c) => RingBufferRole::Consumer(c.clone()),
        }
    }
}

struct RingBuffer {
    producer: RingBufferRole,
    consumer: RingBufferRole,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let rb = HeapRb::<f32>::new(capacity);
        let (producer, consumer) = rb.split();

        RingBuffer {
            producer: RingBufferRole::Producer(Arc::new(Mutex::new(producer))),
            consumer: RingBufferRole::Consumer(Arc::new(Mutex::new(consumer))),
        }
    }
}

enum PortType {
    Input,
    Output,
}

/// Supports single enabled device
struct AudioPort {
    port_type: PortType,
    devices: Vec<cpal::Device>,
    enabled_device_index: Option<usize>,
    stream: Option<cpal::Stream>,
    buffer: RingBufferRole,
}

impl AudioPort {
    pub fn new(
        host: &cpal::Host,
        buffer: &RingBuffer,
        sample_rate: u32,
        port_type: PortType,
    ) -> Self {
        let mut devices: Vec<cpal::Device>;
        let mut enabled_device_index: usize = 0;
        let mut stream: cpal::Stream;
        let mut shared_buffer_ptr: RingBufferRole;

        match port_type {
            PortType::Input => {
                // two copies, one to store for future stream creation, one to be consumed by stream creation now.
                shared_buffer_ptr = buffer.producer.clone();
                let producer = buffer.producer.clone();

                devices = host.input_devices().unwrap().collect();

                // get default device index
                let default_device = host.default_input_device().unwrap();
                for (i, d) in devices.iter().enumerate() {
                    if d.name().unwrap() == default_device.name().unwrap() {
                        enabled_device_index = i;
                    }
                }

                stream = Self::build_stream(&default_device, producer);
                stream.pause().unwrap();
            }

            PortType::Output => {
                shared_buffer_ptr = buffer.consumer.clone();
                let consumer = buffer.consumer.clone();

                devices = host.output_devices().unwrap().collect();

                let default_device = host.default_output_device().unwrap();
                for (i, d) in devices.iter().enumerate() {
                    if d.name().unwrap() == default_device.name().unwrap() {
                        enabled_device_index = i;
                    }
                }

                stream = Self::build_stream(&default_device, consumer);
                stream.pause().unwrap();
            }
        }

        AudioPort {
            port_type,
            devices,
            enabled_device_index: Some(enabled_device_index),
            stream: Some(stream),
            buffer: shared_buffer_ptr,
        }
    }

    fn open_stream(&self) {
        self.stream
            .as_ref()
            .expect("No stream found")
            .play()
            .unwrap()
    }

    fn close_stream(&self) {
        self.stream
            .as_ref()
            .expect("No stream found")
            .pause()
            .unwrap()
    }

    fn get_device_names(&self) -> Vec<String> {
        self.devices.iter().map(|d| d.name().unwrap()).collect()
    }

    fn get_enabled_device_index(&self) -> usize {
        match self.enabled_device_index {
            Some(i) => i,
            None => 0,
        }
    }

    fn set_enabled_device_index(&mut self, index: usize) {
        self.enabled_device_index = Some(index);
        let device = self
            .devices
            .get(index)
            .expect("Could not find device to enable");

        self.stream = Some(Self::build_stream(device, self.buffer.clone()));
    }

    fn build_stream(device: &cpal::Device, shared_buffer_ptr: RingBufferRole) -> cpal::Stream {
        let sample_rate = 44100;
        let buffer_size = 512;
        let num_channels = 2;
        let enabled_channel: usize = 0;
        assert!(enabled_channel < num_channels as usize);

        let latency = 500.0; // delay in ms
        let latency_frames = (latency / 1000.0) * sample_rate as f32;
        let latency_samples = latency_frames as usize * num_channels as usize;

        match shared_buffer_ptr {
            RingBufferRole::Producer(p) => {
                let config = StreamConfig {
                    channels: num_channels,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Fixed(buffer_size),
                };

                for _ in 0..latency_samples {
                    p.lock().unwrap().push(0.0).unwrap();
                }

                let mut fft_planner = FftPlanner::new();

                let process_in_data = move |data: &[f32], _: &InputCallbackInfo| {
                    let mut output_fell_behind = false;
                    let fft_size = data.len() / num_channels as usize;
                    println!("data at 5 {}", data[5]);

                    // forward fft
                    let forward_fft =
                        fft_planner.plan_fft(fft_size, rustfft::FftDirection::Forward);

                    let mut fft_buffer = data
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i % num_channels as usize == enabled_channel)
                        .map(|(_, &d)| Complex::new(d, 0.0))
                        .collect::<Vec<Complex<f32>>>();

                    forward_fft.process(&mut fft_buffer);

                    // apply frequency domain processing

                    // inverse fft
                    let inverse_fft =
                        fft_planner.plan_fft(fft_size, rustfft::FftDirection::Inverse);

                    inverse_fft.process(&mut fft_buffer);
                    for n in fft_buffer.iter_mut() {
                        *n *= 1.0 / ((fft_size as f32).sqrt() * (fft_size as f32).sqrt());
                    }

                    println!("fft_buffer at 5 {}", fft_buffer[5]);

                    // apply time domain processing

                    // push data to shared buffer (duplicated for L/R output)

                    let mut guard = p.lock().expect("Could not aquire lock");
                    for _ in 0..2 {
                        let num_pushed =
                            guard.push_iter(&mut fft_buffer.iter().map(|&d| d.re).into_iter());
                        if num_pushed != fft_buffer.len() {
                            output_fell_behind = true;
                        }
                    }

                    if output_fell_behind {
                        eprintln!("Output stream fell behind; try increasing latency")
                    }
                };

                let stream = device
                    .build_input_stream(
                        &config,
                        process_in_data,
                        |_e| panic!("Could not init input stream"),
                        None,
                    )
                    .expect("Could not build input stream");
                stream
            }
            RingBufferRole::Consumer(c) => {
                let config = StreamConfig {
                    channels: num_channels,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Fixed(buffer_size),
                };

                let process_out_data = move |data: &mut [f32], _: &OutputCallbackInfo| {
                    let mut input_fell_behind = false;
                    let mut guard = c.lock().expect("Could not aquire lock");
                    for d in data {
                        *d = match guard.pop() {
                            Some(s) => s,
                            None => {
                                input_fell_behind = true;
                                0.0
                            }
                        };
                    }
                    if input_fell_behind {
                        eprintln!("Input stream fell behind; try increasing latency")
                    }
                };
                let stream = device
                    .build_output_stream(
                        &config,
                        process_out_data,
                        |_e| panic!("Could not init output stream"),
                        None,
                    )
                    .expect("Could not build output stream");
                stream
            }
        }
    }
}

pub struct IOManager {
    host: cpal::Host,
    output_buffer: RingBuffer,
    output_port: AudioPort,
    input_port: AudioPort,
    pub sample_rate: u32,
}

impl IOManager {
    pub fn new() -> Self {
        let host = cpal::default_host();

        let sample_rate = 44100;
        let num_channels = 2;

        let latency = 500.0; // delay in ms
        let latency_frames = (latency / 1000.0) * sample_rate as f32;
        let latency_samples = latency_frames as usize * num_channels as usize;

        // ring buffer space is twice the necessary size for the stream to make room for latency
        let output_buffer = RingBuffer::new(latency_samples * 2);
        let output_port = AudioPort::new(&host, &output_buffer, 44100, PortType::Output);
        let input_port = AudioPort::new(&host, &output_buffer, 44100, PortType::Input);
        let sample_rate = 44100;

        IOManager {
            host,
            output_buffer,
            output_port,
            input_port,
            sample_rate,
        }
    }

    /// Get a list of input devices for display
    pub fn get_input_device_names(&self) -> Vec<String> {
        self.input_port.get_device_names()
    }

    /// Get a list of output devices for display
    pub fn get_output_devices_names(&self) -> Vec<String> {
        self.output_port.get_device_names()
    }

    pub fn get_current_in_device_index(&self) -> usize {
        self.input_port.get_enabled_device_index()
    }

    pub fn get_current_out_device_index(&self) -> usize {
        self.output_port.get_enabled_device_index()
    }

    // TODO: streams should be rebuilt. this will update sample rate properly
    pub fn set_sample_rate(&mut self, new_sample_rate: u32) {
        self.sample_rate = new_sample_rate;
    }

    /// Builds a stream for the divice found at index.
    pub fn enable_output_device(&mut self, index: usize) {
        self.output_port.set_enabled_device_index(index);
    }

    pub fn enable_input_device(&mut self, index: usize) {
        self.input_port.set_enabled_device_index(index);
    }

    pub fn play_output(&self) {
        self.output_port.open_stream();
    }

    pub fn pause_output(&self) {
        self.output_port.close_stream();
    }

    pub fn play_input(&self) {
        self.input_port.open_stream();
    }

    pub fn pause_input(&self) {
        self.input_port.close_stream();
    }
}

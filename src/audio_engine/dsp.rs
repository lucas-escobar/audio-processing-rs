// const MIN_VCO_HZ: f32 = 20.0;
// const MAX_VCO_HZ: f32 = 20000.0;
// const MAX_LFO_HZ: f32 = 20.0;

// enum WaveShape {
//     Sine,
//     Saw,
//     Square,
//     Triangle,
// }

// struct Envelope {
//     attack: f32,
//     decay: f32,
//     sustain: f32,
//     release: f32,
// }

// enum FilterType {
//     LowPass,
//     HighPass,
//     BandPass,
//     Notch,
//     Peak,
//     LowShelf,
//     HighShelf,
// }

// struct Filter {
//     filter_type: FilterType,
//     cutoff_frequency: f32, // might be variable
//     bandwidth: Option<f32>,
//     resonance: f32,
//     drive: f32,
//     slope: f32,
//     gain: f32,
//     wet_dry_mix: f32,
// }

// struct VoltageControlledOscillator {
//     pitch: f32,
//     lfo_voltage: f32,
//     envelope: Envelope,
// }

// struct LowFrequencyOscillator {
//     pitch: f32,
//     wave_shape: WaveShape,
// }

// enum NoiseType {
//     White,
//     Pink,
//     Brownian,
//     Blue,
//     Violet,
//     Gray,
//     Green,
//     Binary,
//     Gaussian,
// }

// struct NoiseGenerator {
//     noise_type: NoiseType,
//     resample_rate: f32,
//     bit_depth: u32,
//     seed: u64,
//     stereo: bool,
//     amplitude: f32,
//     frequency: f32,
//     frequency_mod: f32,
//     envelope: Envelope,
//     filter: Option<Filter>,
// }

//! # Overview
//! 
//! `karbeat-dsp` is a building block collection of common
//! operation in Digital Audio Signal Processing.
//! This crate provides implementation for generally used DSP
//! such as Oscillator, Filter, Pitch Shifting, Time Stretcher, etc
//! 
//! # Examples
//! 
//! For an example of a use case, you can consider this example,
//! where you want to use Oscillator struct to do heavy-lifting
//! for your audio synthesis:
//! 
//! ```rust, ignore
//! import karbeat_dsp::prelude::*
//! 
//! let osc = Oscillator(1, "MyOsc");
//! 
//! let mut out_buffer = Vec::with_capacity(512);
//! let mut phase = 0.0;
//! 
//! // Do something with your osc
//! osc.output_wave(&mut out_buffer, 44100, 2, 440.0, &mut phase);
//! ```

pub mod interpolation;
pub mod envelope;
pub mod oscillator;
pub mod bit_crush;
pub mod chorus;
pub mod filter;
pub mod flanger;
pub mod pitch_shift;
pub mod reverb;
pub mod stretcher;

pub mod prelude;
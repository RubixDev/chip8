use std::f32::consts::PI;

use rodio::Source;

pub struct SquareWave {
    freq: f32,
    sample_idx: usize,
}

impl SquareWave {
    pub fn new(freq: f32) -> Self {
        Self {
            freq,
            sample_idx: 0,
        }
    }
}

impl Iterator for SquareWave {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.sample_idx = self.sample_idx.wrapping_add(1);
        let value = 2.0 * PI * self.freq * self.sample_idx as f32 / 48000.0;
        Some(value.sin().signum())
    }
}

impl Source for SquareWave {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        48000
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

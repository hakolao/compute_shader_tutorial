use std::{collections::VecDeque, time::Instant};

const NUM_TIME_SAMPLES: usize = 150;

/// A simple performance timer with a buffer of delta times to track performance over time
pub struct PerformanceTimer {
    time: Instant,
    data: VecDeque<f64>,
}

impl PerformanceTimer {
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
            data: VecDeque::new(),
        }
    }

    pub fn start(&mut self) {
        self.time = Instant::now()
    }

    #[allow(unused)]
    pub fn end(&self) -> f64 {
        Instant::now().duration_since(self.time).as_nanos() as f64 / 1_000_000.0
    }

    pub fn time_it(&mut self) {
        let time = Instant::now().duration_since(self.time).as_nanos() as f64 / 1_000_000.0;
        self.data.push_back(time);
        if self.data.len() >= NUM_TIME_SAMPLES {
            self.data.pop_front();
        }
    }

    #[allow(unused)]
    pub fn push_dt_ms(&mut self, dt: f64) {
        self.data.push_back(dt);
        if self.data.len() >= NUM_TIME_SAMPLES {
            self.data.pop_front();
        }
    }

    pub fn time_average_ms(&self) -> f64 {
        self.data.iter().sum::<f64>() / self.data.len() as f64
    }
}

impl Default for PerformanceTimer {
    fn default() -> Self {
        PerformanceTimer::new()
    }
}

pub struct SimTimer(pub PerformanceTimer);

pub struct RenderTimer(pub PerformanceTimer);

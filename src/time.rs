use std::time::{Duration, Instant};

pub struct Time {
    delta: Duration,
    delta_seconds: f64,
    last_update: Instant,
}

impl Time {
    pub fn new() -> Time {
        Time {
            delta: Duration::from_secs(0),
            delta_seconds: 0.0,
            last_update: Instant::now(),
        }
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds as f32
    }

    pub fn update(&mut self) {
        let delta_time = self.last_update.elapsed();
        self.last_update = Instant::now();

        self.delta = delta_time;
        self.delta_seconds = delta_time.as_secs_f64();
    }
}

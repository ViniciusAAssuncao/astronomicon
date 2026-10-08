use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TickStageTiming {
    pub name: &'static str,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct TickProfile {
    last_mark: Instant,
    stages: Vec<TickStageTiming>,
}

impl TickProfile {
    pub(super) fn new() -> Self {
        Self {
            last_mark: Instant::now(),
            stages: Vec::with_capacity(8),
        }
    }

    pub(super) fn mark(&mut self, name: &'static str) {
        let now = Instant::now();
        self.stages.push(TickStageTiming {
            name,
            duration: now.duration_since(self.last_mark),
        });
        self.last_mark = now;
    }

    pub fn stages(&self) -> &[TickStageTiming] {
        &self.stages
    }
}

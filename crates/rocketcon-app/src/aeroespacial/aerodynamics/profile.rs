use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AerodynamicsStageTiming {
    pub name: &'static str,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct AerodynamicsProfile {
    last_mark: Instant,
    stages: Vec<AerodynamicsStageTiming>,
}

impl AerodynamicsProfile {
    pub(super) fn new() -> Self {
        Self {
            last_mark: Instant::now(),
            stages: Vec::with_capacity(5),
        }
    }

    pub(super) fn mark(&mut self, name: &'static str) {
        let now = Instant::now();
        self.stages.push(AerodynamicsStageTiming {
            name,
            duration: now.duration_since(self.last_mark),
        });
        self.last_mark = now;
    }

    pub fn stages(&self) -> &[AerodynamicsStageTiming] {
        &self.stages
    }
}

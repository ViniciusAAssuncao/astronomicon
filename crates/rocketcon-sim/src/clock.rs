use astronomicon_core::units::Duration;

pub struct SimulationClock {
    universe_epoch: Duration,
    at_epoch: Duration,
}

impl SimulationClock {
    pub fn new(universe_epoch: Duration) -> Self {
        Self {
            universe_epoch,
            at_epoch: Duration::new(0.0),
        }
    }

    pub fn tick(&mut self, dt: Duration) {
        self.at_epoch = self.at_epoch + dt;
    }

    pub fn universe_epoch(&self) -> Duration {
        self.universe_epoch
    }

    pub fn at_epoch(&self) -> Duration {
        self.at_epoch
    }

    pub fn total_epoch(&self) -> Duration {
        self.universe_epoch + self.at_epoch
    }
}

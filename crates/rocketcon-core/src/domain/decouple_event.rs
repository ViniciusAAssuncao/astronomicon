use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecoupleEvent {
    stage_index: u32,
}

impl DecoupleEvent {
    pub fn new(stage_index: u32) -> Self {
        Self { stage_index }
    }

    pub fn stage_index(&self) -> u32 {
        self.stage_index
    }

    pub fn apply(&self, active_stages: &[u32]) -> Vec<u32> {
        active_stages
            .iter()
            .copied()
            .filter(|&stage| stage != self.stage_index)
            .collect()
    }

    pub fn apply_to_active_stages(&self, active_stages: &[u32]) -> Vec<u32> {
        self.apply(active_stages)
    }
}
use astronomicon_core::units::{Duration, Length, Speed, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ManeuverDeltaV {
    pub prograde: Speed,
    pub normal: Speed,
    pub radial: Speed,
}

impl ManeuverDeltaV {
    pub fn new(prograde: Speed, normal: Speed, radial: Speed) -> Self {
        Self {
            prograde,
            normal,
            radial,
        }
    }

    pub fn zero() -> Self {
        Self {
            prograde: Speed::new(0.0),
            normal: Speed::new(0.0),
            radial: Speed::new(0.0),
        }
    }

    pub fn prograde(prograde: Speed) -> Self {
        Self {
            prograde,
            normal: Speed::new(0.0),
            radial: Speed::new(0.0),
        }
    }

    pub fn normal(normal: Speed) -> Self {
        Self {
            prograde: Speed::new(0.0),
            normal,
            radial: Speed::new(0.0),
        }
    }

    pub fn radial(radial: Speed) -> Self {
        Self {
            prograde: Speed::new(0.0),
            normal: Speed::new(0.0),
            radial,
        }
    }

    pub fn total_magnitude(&self) -> Speed {
        let p = self.prograde.value();
        let n = self.normal.value();
        let r = self.radial.value();
        Speed::new((p * p + n * n + r * r).sqrt())
    }

    pub fn to_vector3(&self) -> Vector3 {
        Vector3::new(
            self.prograde.value(),
            self.normal.value(),
            self.radial.value(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ManeuverNode {
    pub scheduled_epoch: Duration,
    pub delta_v: ManeuverDeltaV,
    pub estimated_burn_duration: Duration,
}

impl ManeuverNode {
    pub fn new(
        scheduled_epoch: Duration,
        delta_v: ManeuverDeltaV,
        estimated_burn_duration: Duration,
    ) -> Self {
        Self {
            scheduled_epoch,
            delta_v,
            estimated_burn_duration,
        }
    }

    pub fn scheduled_epoch(&self) -> Duration {
        self.scheduled_epoch
    }

    pub fn delta_v(&self) -> ManeuverDeltaV {
        self.delta_v
    }

    pub fn estimated_burn_duration(&self) -> Duration {
        self.estimated_burn_duration
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HohmannTransferResult {
    pub delta_v_departure: Speed,
    pub delta_v_arrival: Speed,
    pub total_delta_v: Speed,
    pub time_of_flight: Duration,
    pub transfer_semi_major_axis: Length,
}

impl HohmannTransferResult {
    pub fn new(
        delta_v_departure: Speed,
        delta_v_arrival: Speed,
        total_delta_v: Speed,
        time_of_flight: Duration,
        transfer_semi_major_axis: Length,
    ) -> Self {
        Self {
            delta_v_departure,
            delta_v_arrival,
            total_delta_v,
            time_of_flight,
            transfer_semi_major_axis,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiEllipticTransferResult {
    pub delta_v_1: Speed,
    pub delta_v_2: Speed,
    pub delta_v_3: Speed,
    pub total_delta_v: Speed,
    pub time_of_flight: Duration,
    pub first_transfer_semi_major_axis: Length,
    pub second_transfer_semi_major_axis: Length,
}

impl BiEllipticTransferResult {
    pub fn new(
        delta_v_1: Speed,
        delta_v_2: Speed,
        delta_v_3: Speed,
        total_delta_v: Speed,
        time_of_flight: Duration,
        first_transfer_semi_major_axis: Length,
        second_transfer_semi_major_axis: Length,
    ) -> Self {
        Self {
            delta_v_1,
            delta_v_2,
            delta_v_3,
            total_delta_v,
            time_of_flight,
            first_transfer_semi_major_axis,
            second_transfer_semi_major_axis,
        }
    }
}
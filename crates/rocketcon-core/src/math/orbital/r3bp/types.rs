use astronomicon_core::units::{Duration, Length, Mass, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LibrationPoint {
    L1,
    L2,
    L3,
    L4,
    L5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HaloOrbitFamily {
    Northern,
    Southern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManifoldType {
    Stable,
    Unstable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManifoldDirection {
    Interior,
    Exterior,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cr3bpParameters {
    pub mu: f64,
    pub characteristic_length: Length,
    pub characteristic_mass: Mass,
    pub characteristic_time: Duration,
    pub primary_mass: Mass,
    pub secondary_mass: Mass,
}

impl Cr3bpParameters {
    pub fn new(primary_mass: Mass, secondary_mass: Mass, separation: Length) -> Self {
        let m1 = primary_mass.value();
        let m2 = secondary_mass.value();
        let l = separation.value();
        let m_tot = m1 + m2;
        let mu = if m_tot > 0.0 { m2 / m_tot } else { 0.0 };
        let g = astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
        let t_star = if m_tot > 0.0 && l > 0.0 {
            (l.powi(3) / (g * m_tot)).sqrt()
        } else {
            0.0
        };

        Self {
            mu,
            characteristic_length: separation,
            characteristic_mass: Mass::new(m_tot),
            characteristic_time: Duration::new(t_star),
            primary_mass,
            secondary_mass,
        }
    }

    pub fn primary_position_synodic(&self) -> Vector3 {
        Vector3::new(-self.mu, 0.0, 0.0)
    }

    pub fn secondary_position_synodic(&self) -> Vector3 {
        Vector3::new(1.0 - self.mu, 0.0, 0.0)
    }

    pub fn angular_velocity(&self) -> f64 {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Er3bpParameters {
    pub cr3bp: Cr3bpParameters,
    pub eccentricity: f64,
}

impl Er3bpParameters {
    pub fn new(cr3bp: Cr3bpParameters, eccentricity: f64) -> Self {
        Self {
            cr3bp,
            eccentricity: eccentricity.clamp(0.0, 0.999),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SynodicState {
    pub position: Vector3,
    pub velocity: Vector3,
}

impl SynodicState {
    pub fn new(position: Vector3, velocity: Vector3) -> Self {
        Self { position, velocity }
    }

    pub fn from_components(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64) -> Self {
        Self {
            position: Vector3::new(x, y, z),
            velocity: Vector3::new(vx, vy, vz),
        }
    }

    pub fn zero() -> Self {
        Self {
            position: Vector3::zero(),
            velocity: Vector3::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaloOrbitState {
    pub initial_state: SynodicState,
    pub period_dimensionless: f64,
    pub jacobi_constant: f64,
    pub libration_point: LibrationPoint,
    pub family: HaloOrbitFamily,
    pub az_amplitude_dimensionless: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifoldTrajectory {
    pub manifold_type: ManifoldType,
    pub direction: ManifoldDirection,
    pub states: Vec<SynodicState>,
    pub times_dimensionless: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifoldTube {
    pub manifold_type: ManifoldType,
    pub direction: ManifoldDirection,
    pub trajectories: Vec<ManifoldTrajectory>,
}
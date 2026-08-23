use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrbitalParent {
    Star(Uuid),
    Planet(Uuid),
    Barycenter(Uuid),
    MinorPlanet(Uuid),
    Fixed,
}
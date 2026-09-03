use crate::domain::battery_specification::BatterySpecification;
use crate::domain::component::Component;
use crate::domain::engine_specification::EngineSpecification;
use crate::domain::heat_shield_specification::HeatShieldSpecification;
use crate::domain::hull_specification::HullSpecification;
use crate::domain::nuclear_reactor_specification::NuclearReactorSpecification;
use crate::domain::payload_specification::PayloadSpecification;
use crate::domain::propellant_tank_specification::PropellantTankSpecification;
use crate::domain::radiator_specification::RadiatorSpecification;
use crate::domain::reaction_control_thruster_specification::ReactionControlThrusterSpecification;
use crate::domain::reaction_wheel_specification::ReactionWheelSpecification;
use crate::domain::rtg_specification::RtgSpecification;
use crate::domain::solar_panel_specification::SolarPanelSpecification;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentDetails {
    Engine(EngineSpecification),
    PropellantTank(PropellantTankSpecification),
    Battery(BatterySpecification),
    SolarPanel(SolarPanelSpecification),
    Cpu,
    ReactionControlThruster(ReactionControlThrusterSpecification),
    ReactionWheel(ReactionWheelSpecification),
    Rtg(RtgSpecification),
    NuclearReactor(NuclearReactorSpecification),
    Radiator(RadiatorSpecification),
    Payload(PayloadSpecification),
    Hull(HullSpecification),
    HeatShield(HeatShieldSpecification),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentRecord {
    component: Component,
    details: ComponentDetails,
}

impl ComponentRecord {
    pub fn new(component: Component, details: ComponentDetails) -> Self {
        Self { component, details }
    }

    pub fn component(&self) -> &Component {
        &self.component
    }

    pub fn details(&self) -> &ComponentDetails {
        &self.details
    }
}
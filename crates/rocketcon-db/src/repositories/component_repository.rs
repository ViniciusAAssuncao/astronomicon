use crate::error::{RocketDbError, RocketDbResult};
use crate::models::ComponentRow;
use crate::repositories::component_attributes::{
    fetch_attribute_map, optional_numeric, optional_uuid, required_bool, required_numeric,
    required_text, required_uuid,
};
use astronomicon_core::units::{
    Angle, AngularMomentum, AngularVelocity, Duration, Energy, Force, Impulse, Luminosity, Mass,
    Speed, Torque, Volume,
};
use rocketcon_core::domain::{
    BatterySpecification, Component, ComponentDetails, ComponentKind, ComponentRecord,
    EngineSpecification, IgnitionType, NuclearReactorSpecification, NuclearReactorType,
    PayloadSpecification, PropellantTankSpecification, RadiatorSpecification,
    ReactionControlThrusterSpecification, ReactionWheelSpecification, RtgSpecification,
    SolarPanelSpecification,
};
use rocketcon_core::error::RocketDomainError;
use rocketcon_core::physics_reference::{NuclearFuelType, RadioisotopeType};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str =
    "SELECT id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w, manufacturer, manufactured_at_unix_seconds, lore_notes FROM components";

async fn fetch_component_details(
    pool: &SqlitePool,
    component: &Component,
) -> RocketDbResult<ComponentDetails> {
    let id = component.id();
    if component.kind() == ComponentKind::Cpu {
        return Ok(ComponentDetails::Cpu);
    }

    let attr_map = fetch_attribute_map(pool, &id).await?;

    match component.kind() {
        ComponentKind::Cpu => Ok(ComponentDetails::Cpu),
        ComponentKind::Engine => {
            let fuel_propellant_id = required_uuid(&attr_map, &id, "fuel_propellant_id")?;
            let specific_impulse_vacuum_s =
                required_numeric(&attr_map, &id, "specific_impulse_vacuum_s")?;
            let max_thrust_n = required_numeric(&attr_map, &id, "max_thrust_n")?;
            let ignition_type_str = required_text(&attr_map, &id, "ignition_type")?;
            let ignition_type = match ignition_type_str {
                "Restartable" => IgnitionType::Restartable,
                "SingleBurn" => IgnitionType::SingleBurn,
                other => {
                    return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "ignition_type".to_string(),
                        reason: format!("unknown ignition type: {}", other),
                    }));
                }
            };
            let oxidizer_propellant_id =
                optional_uuid(&attr_map, &id, "oxidizer_propellant_id")?;
            let specific_impulse_sea_level_s =
                optional_numeric(&attr_map, &id, "specific_impulse_sea_level_s")?;
            let integral_propellant_mass_kg =
                optional_numeric(&attr_map, &id, "integral_propellant_mass_kg")?;
            let max_gimbal_deflection_rad =
                optional_numeric(&attr_map, &id, "max_gimbal_deflection_rad")?;
            let gimbal_slew_rate_rad_s =
                optional_numeric(&attr_map, &id, "gimbal_slew_rate_rad_s")?;
            let min_throttle_fraction =
                optional_numeric(&attr_map, &id, "min_throttle_fraction")?;
            let oxidizer_to_fuel_mass_ratio =
                optional_numeric(&attr_map, &id, "oxidizer_to_fuel_mass_ratio")?;

            let spec = EngineSpecification::builder(
                id,
                fuel_propellant_id,
                Duration::new(specific_impulse_vacuum_s),
                Force::new(max_thrust_n),
                ignition_type,
            )
            .with_oxidizer_propellant_id(oxidizer_propellant_id)
            .with_specific_impulse_sea_level(
                specific_impulse_sea_level_s.map(Duration::new),
            )
            .with_integral_propellant_mass(integral_propellant_mass_kg.map(Mass::new))
            .with_max_gimbal_deflection(max_gimbal_deflection_rad.map(Angle::new))
            .with_gimbal_slew_rate(gimbal_slew_rate_rad_s.map(AngularVelocity::new))
            .with_min_throttle_fraction(min_throttle_fraction)
            .with_oxidizer_to_fuel_mass_ratio(oxidizer_to_fuel_mass_ratio)
            .build()?;

            Ok(ComponentDetails::Engine(spec))
        }
        ComponentKind::PropellantTank => {
            let propellant_id = required_uuid(&attr_map, &id, "propellant_id")?;
            let max_propellant_mass_kg =
                required_numeric(&attr_map, &id, "max_propellant_mass_kg")?;

            let spec = PropellantTankSpecification::new(
                id,
                propellant_id,
                Mass::new(max_propellant_mass_kg),
            )?;

            Ok(ComponentDetails::PropellantTank(spec))
        }
        ComponentKind::Battery => {
            let capacity_j = required_numeric(&attr_map, &id, "capacity_j")?;
            let max_discharge_power_w =
                required_numeric(&attr_map, &id, "max_discharge_power_w")?;
            let max_charge_power_w =
                optional_numeric(&attr_map, &id, "max_charge_power_w")?;

            let spec = BatterySpecification::new(
                id,
                Energy::new(capacity_j),
                Luminosity::new(max_discharge_power_w),
                max_charge_power_w.map(Luminosity::new),
            )?;

            Ok(ComponentDetails::Battery(spec))
        }
        ComponentKind::SolarPanel => {
            let surface_area_m2 = required_numeric(&attr_map, &id, "surface_area_m2")?;
            let conversion_efficiency =
                required_numeric(&attr_map, &id, "conversion_efficiency")?;
            let max_power_output_w =
                required_numeric(&attr_map, &id, "max_power_output_w")?;
            let is_sun_tracking = required_bool(&attr_map, &id, "is_sun_tracking")?;
            let solar_absorptivity =
                optional_numeric(&attr_map, &id, "solar_absorptivity")?;

            let spec = SolarPanelSpecification::new(
                id,
                surface_area_m2,
                conversion_efficiency,
                Luminosity::new(max_power_output_w),
                is_sun_tracking,
                solar_absorptivity,
            )?;

            Ok(ComponentDetails::SolarPanel(spec))
        }
        ComponentKind::ReactionControlThruster => {
            let propellant_id = required_uuid(&attr_map, &id, "propellant_id")?;
            let specific_impulse_vacuum_s =
                required_numeric(&attr_map, &id, "specific_impulse_vacuum_s")?;
            let max_thrust_n = required_numeric(&attr_map, &id, "max_thrust_n")?;
            let min_impulse_bit_n_s =
                optional_numeric(&attr_map, &id, "min_impulse_bit_n_s")?;

            let spec = ReactionControlThrusterSpecification::new(
                id,
                propellant_id,
                Duration::new(specific_impulse_vacuum_s),
                Force::new(max_thrust_n),
                min_impulse_bit_n_s.map(Impulse::new),
            )?;

            Ok(ComponentDetails::ReactionControlThruster(spec))
        }
        ComponentKind::ReactionWheel => {
            let max_torque_n_m = required_numeric(&attr_map, &id, "max_torque_n_m")?;
            let max_angular_momentum_storage_n_m_s =
                required_numeric(&attr_map, &id, "max_angular_momentum_storage_n_m_s")?;

            let spec = ReactionWheelSpecification::new(
                id,
                Torque::new(max_torque_n_m),
                AngularMomentum::new(max_angular_momentum_storage_n_m_s),
            )?;

            Ok(ComponentDetails::ReactionWheel(spec))
        }
        ComponentKind::Rtg => {
            let isotope_str = required_text(&attr_map, &id, "radioisotope")?;
            let radioisotope = match isotope_str {
                "Plutonium238" => RadioisotopeType::Plutonium238,
                "Americium241" => RadioisotopeType::Americium241,
                "Strontium90" => RadioisotopeType::Strontium90,
                "Polonium210" => RadioisotopeType::Polonium210,
                "Curium244" => RadioisotopeType::Curium244,
                "Curium242" => RadioisotopeType::Curium242,
                other => {
                    return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "radioisotope".to_string(),
                        reason: format!("unknown radioisotope: {}", other),
                    }));
                }
            };
            let fuel_mass_kg = required_numeric(&attr_map, &id, "fuel_mass_kg")?;
            let conversion_efficiency =
                required_numeric(&attr_map, &id, "conversion_efficiency")?;
            let fuel_loaded_universe_epoch_s =
                required_numeric(&attr_map, &id, "fuel_loaded_universe_epoch_s")?;

            let spec = RtgSpecification::new(
                id,
                radioisotope,
                Mass::new(fuel_mass_kg),
                conversion_efficiency,
                Duration::new(fuel_loaded_universe_epoch_s),
            )?;

            Ok(ComponentDetails::Rtg(spec))
        }
        ComponentKind::NuclearReactor => {
            let reactor_type_str = required_text(&attr_map, &id, "reactor_type")?;
            let reactor_type = match reactor_type_str {
                "Fission" => NuclearReactorType::Fission,
                "Fusion" => NuclearReactorType::Fusion,
                other => {
                    return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "reactor_type".to_string(),
                        reason: format!("unknown reactor type: {}", other),
                    }));
                }
            };
            let fuel_type_str = required_text(&attr_map, &id, "fuel_type")?;
            let fuel_type = match fuel_type_str {
                "Uranium235" => NuclearFuelType::Uranium235,
                "Plutonium239" => NuclearFuelType::Plutonium239,
                "DeuteriumTritium" => NuclearFuelType::DeuteriumTritium,
                "DeuteriumDeuterium" => NuclearFuelType::DeuteriumDeuterium,
                other => {
                    return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "fuel_type".to_string(),
                        reason: format!("unknown nuclear fuel type: {}", other),
                    }));
                }
            };
            let fuel_mass_kg = required_numeric(&attr_map, &id, "fuel_mass_kg")?;
            let max_thermal_power_w =
                required_numeric(&attr_map, &id, "max_thermal_power_w")?;
            let conversion_efficiency =
                required_numeric(&attr_map, &id, "conversion_efficiency")?;
            let min_throttle_fraction =
                optional_numeric(&attr_map, &id, "min_throttle_fraction")?;

            let spec = NuclearReactorSpecification::new(
                id,
                reactor_type,
                fuel_type,
                Mass::new(fuel_mass_kg),
                Luminosity::new(max_thermal_power_w),
                conversion_efficiency,
                min_throttle_fraction,
            )?;

            Ok(ComponentDetails::NuclearReactor(spec))
        }
        ComponentKind::Radiator => {
            let radiating_area_m2 = required_numeric(&attr_map, &id, "radiating_area_m2")?;
            let emissivity = required_numeric(&attr_map, &id, "emissivity")?;
            let solar_absorptivity = required_numeric(&attr_map, &id, "solar_absorptivity")?;

            let spec = RadiatorSpecification::new(
                id,
                radiating_area_m2,
                emissivity,
                solar_absorptivity,
            )?;

            Ok(ComponentDetails::Radiator(spec))
        }
        ComponentKind::PayloadFairing | ComponentKind::PayloadDispenser => {
            let max_payload_mass_kg = match optional_numeric(&attr_map, &id, "max_payload_mass_kg")? {
                Some(v) => v,
                None => required_numeric(&attr_map, &id, "max_payload_mass")?,
            };
            let max_payload_volume_m3 =
                match optional_numeric(&attr_map, &id, "max_payload_volume_m3")? {
                    Some(v) => v,
                    None => required_numeric(&attr_map, &id, "max_payload_volume")?,
                };
            let contained_vehicle_id = optional_uuid(&attr_map, &id, "contained_vehicle_id")?;
            let generic_cargo_mass_kg =
                match optional_numeric(&attr_map, &id, "generic_cargo_mass_kg")? {
                    Some(v) => Some(v),
                    None => optional_numeric(&attr_map, &id, "generic_cargo_mass")?,
                };
            let separation_velocity_m_s =
                match optional_numeric(&attr_map, &id, "separation_velocity_m_s")? {
                    Some(v) => v,
                    None => match optional_numeric(&attr_map, &id, "separation_velocity")? {
                        Some(v) => v,
                        None => 0.0,
                    },
                };

            let spec = PayloadSpecification::builder(
                id,
                Mass::new(max_payload_mass_kg),
                Volume::new(max_payload_volume_m3),
                Speed::new(separation_velocity_m_s),
            )
            .with_contained_vehicle_id(contained_vehicle_id)
            .with_generic_cargo_mass(generic_cargo_mass_kg.map(Mass::new))
            .build()?;

            Ok(ComponentDetails::Payload(spec))
        }
    }
}

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<ComponentRecord>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, ComponentRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    let component_row = match row {
        Some(r) => r,
        None => {
            return Ok(None);
        }
    };

    let component = Component::try_from(component_row)?;
    let details = fetch_component_details(pool, &component).await?;

    Ok(Some(ComponentRecord::new(component, details)))
}

pub async fn list_all(pool: &SqlitePool) -> RocketDbResult<Vec<ComponentRecord>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = sqlx::query_as::<_, ComponentRow>(&query)
        .fetch_all(pool)
        .await?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let component = Component::try_from(row)?;
        let details = fetch_component_details(pool, &component).await?;
        records.push(ComponentRecord::new(component, details));
    }

    Ok(records)
}
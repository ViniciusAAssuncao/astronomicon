use crate::error::{RocketDbError, RocketDbResult};
use crate::models::{
    ComponentBatteryRow, ComponentEngineRow, ComponentPropellantTankRow, ComponentRow,
    ComponentSolarPanelRow,
};
use rocketcon_core::domain::{
    BatterySpecification, Component, ComponentDetails, ComponentKind, ComponentRecord,
    EngineSpecification, PropellantTankSpecification, SolarPanelSpecification,
};
use rocketcon_core::error::RocketDomainError;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w, manufacturer, manufactured_at_unix_seconds, lore_notes FROM components";

async fn fetch_component_details(
    pool: &SqlitePool,
    component: &Component,
) -> RocketDbResult<ComponentDetails> {
    let id_str = component.id().to_string();

    match component.kind() {
        ComponentKind::Engine => {
            let row = sqlx::query_as::<_, ComponentEngineRow>(
                "SELECT component_id, fuel_propellant_id, oxidizer_propellant_id, specific_impulse_vacuum_s, specific_impulse_sea_level_s, max_thrust_n, ignition_type, integral_propellant_mass_kg FROM component_engines WHERE component_id = ?",
            )
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_engines".to_string(),
                    reason: format!(
                        "missing engine specification for component id '{}'",
                        component.id()
                    ),
                })
            })?;

            let spec = EngineSpecification::try_from(row)?;
            Ok(ComponentDetails::Engine(spec))
        }
        ComponentKind::PropellantTank => {
            let row = sqlx::query_as::<_, ComponentPropellantTankRow>(
                "SELECT component_id, propellant_id, max_propellant_mass_kg FROM component_propellant_tanks WHERE component_id = ?",
            )
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_propellant_tanks".to_string(),
                    reason: format!(
                        "missing propellant tank specification for component id '{}'",
                        component.id()
                    ),
                })
            })?;

            let spec = PropellantTankSpecification::try_from(row)?;
            Ok(ComponentDetails::PropellantTank(spec))
        }
        ComponentKind::Battery => {
            let row = sqlx::query_as::<_, ComponentBatteryRow>(
                "SELECT component_id, capacity_j, max_discharge_power_w, max_charge_power_w FROM component_batteries WHERE component_id = ?",
            )
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_batteries".to_string(),
                    reason: format!(
                        "missing battery specification for component id '{}'",
                        component.id()
                    ),
                })
            })?;

            let spec = BatterySpecification::try_from(row)?;
            Ok(ComponentDetails::Battery(spec))
        }
        ComponentKind::SolarPanel => {
            let row = sqlx::query_as::<_, ComponentSolarPanelRow>(
                "SELECT component_id, surface_area_m2, conversion_efficiency, max_power_output_w, is_sun_tracking FROM component_solar_panels WHERE component_id = ?",
            )
            .bind(&id_str)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_solar_panels".to_string(),
                    reason: format!(
                        "missing solar panel specification for component id '{}'",
                        component.id()
                    ),
                })
            })?;

            let spec = SolarPanelSpecification::try_from(row)?;
            Ok(ComponentDetails::SolarPanel(spec))
        }
        ComponentKind::Cpu => Ok(ComponentDetails::Cpu),
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
        None => return Ok(None),
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
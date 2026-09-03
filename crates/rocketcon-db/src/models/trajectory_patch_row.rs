use crate::error::RocketDbError;
use astronomicon_core::units::{
    Angle, Duration, Force, GravitationalParameter, Length, Mass, Speed,
};
use rocketcon_core::domain::{
    ConicPatchData, LowThrustPatchData, TrajectoryPatch, TrajectoryPatchKind,
};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct TrajectoryPatchRow {
    pub id: String,
    pub vehicle_id: String,
    pub reference_body_id: String,
    pub start_universe_epoch_s: f64,
    pub end_universe_epoch_s: Option<f64>,
    pub gravitational_parameter_m3_s2: f64,
    pub patch_type: String,
    pub semi_major_axis_m: Option<f64>,
    pub eccentricity: Option<f64>,
    pub inclination_rad: Option<f64>,
    pub longitude_of_ascending_node_rad: Option<f64>,
    pub argument_of_periapsis_rad: Option<f64>,
    pub true_anomaly_at_epoch_rad: Option<f64>,
    pub initial_mass_kg: Option<f64>,
    pub final_mass_kg: Option<f64>,
    pub thrust_n: Option<f64>,
    pub specific_impulse_s: Option<f64>,
    pub total_delta_v_m_s: Option<f64>,
    pub chebyshev_x_json: Option<String>,
    pub chebyshev_y_json: Option<String>,
    pub chebyshev_z_json: Option<String>,
    pub chebyshev_vx_json: Option<String>,
    pub chebyshev_vy_json: Option<String>,
    pub chebyshev_vz_json: Option<String>,
    pub chebyshev_mass_json: Option<String>,
}

pub fn parse_float_list(text: &str) -> Vec<f64> {
    text.trim_matches(|c| c == '[' || c == ']' || c == ' ' || c == '\n' || c == '\r' || c == '\t')
        .split(',')
        .filter_map(|part| part.trim().parse::<f64>().ok())
        .collect()
}

pub fn format_float_list(slice: &[f64]) -> String {
    let mut s = String::from("[");
    for (i, val) in slice.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&val.to_string());
    }
    s.push(']');
    s
}

impl TryFrom<TrajectoryPatchRow> for TrajectoryPatch {
    type Error = RocketDbError;

    fn try_from(row: TrajectoryPatchRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let vehicle_id = Uuid::parse_str(&row.vehicle_id)?;
        let reference_body_id = Uuid::parse_str(&row.reference_body_id)?;
        let start_epoch = Duration::new(row.start_universe_epoch_s);
        let end_epoch = row.end_universe_epoch_s.map(Duration::new);
        let mu = GravitationalParameter::new(row.gravitational_parameter_m3_s2);

        match row.patch_type.as_str() {
            "conic" => {
                let sma = row.semi_major_axis_m.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "semi_major_axis_m".to_string(),
                        reason: "missing required conic field semi_major_axis_m".to_string(),
                    })
                })?;
                let ecc = row.eccentricity.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "eccentricity".to_string(),
                        reason: "missing required conic field eccentricity".to_string(),
                    })
                })?;
                let inc = row.inclination_rad.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "inclination_rad".to_string(),
                        reason: "missing required conic field inclination_rad".to_string(),
                    })
                })?;
                let raan = row.longitude_of_ascending_node_rad.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "longitude_of_ascending_node_rad".to_string(),
                        reason: "missing required conic field longitude_of_ascending_node_rad".to_string(),
                    })
                })?;
                let arg_p = row.argument_of_periapsis_rad.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "argument_of_periapsis_rad".to_string(),
                        reason: "missing required conic field argument_of_periapsis_rad".to_string(),
                    })
                })?;
                let nu = row.true_anomaly_at_epoch_rad.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "true_anomaly_at_epoch_rad".to_string(),
                        reason: "missing required conic field true_anomaly_at_epoch_rad".to_string(),
                    })
                })?;

                let conic = ConicPatchData::new(
                    Length::new(sma),
                    ecc,
                    Angle::new(inc),
                    Angle::new(raan),
                    Angle::new(arg_p),
                    Angle::new(nu),
                )?;

                let patch = TrajectoryPatch::new_with_kind(
                    id,
                    vehicle_id,
                    reference_body_id,
                    start_epoch,
                    end_epoch,
                    mu,
                    TrajectoryPatchKind::Conic(conic),
                )?;
                Ok(patch)
            }
            "low_thrust" => {
                let m0 = row.initial_mass_kg.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "initial_mass_kg".to_string(),
                        reason: "missing required low_thrust field initial_mass_kg".to_string(),
                    })
                })?;
                let mf = row.final_mass_kg.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "final_mass_kg".to_string(),
                        reason: "missing required low_thrust field final_mass_kg".to_string(),
                    })
                })?;
                let thrust = row.thrust_n.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "thrust_n".to_string(),
                        reason: "missing required low_thrust field thrust_n".to_string(),
                    })
                })?;
                let isp = row.specific_impulse_s.ok_or_else(|| {
                    RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                        field: "specific_impulse_s".to_string(),
                        reason: "missing required low_thrust field specific_impulse_s".to_string(),
                    })
                })?;
                let dv = row.total_delta_v_m_s.unwrap_or(0.0);

                let cx = row.chebyshev_x_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cy = row.chebyshev_y_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cz = row.chebyshev_z_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cvx = row.chebyshev_vx_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cvy = row.chebyshev_vy_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cvz = row.chebyshev_vz_json.as_deref().map(parse_float_list).unwrap_or_default();
                let cmass = row.chebyshev_mass_json.as_deref().map(parse_float_list).unwrap_or_default();

                let lt = LowThrustPatchData::new(
                    Mass::new(m0),
                    Mass::new(mf),
                    Force::new(thrust),
                    Duration::new(isp),
                    Speed::new(dv),
                    cx,
                    cy,
                    cz,
                    cvx,
                    cvy,
                    cvz,
                    cmass,
                )?;

                let patch = TrajectoryPatch::new_with_kind(
                    id,
                    vehicle_id,
                    reference_body_id,
                    start_epoch,
                    end_epoch,
                    mu,
                    TrajectoryPatchKind::LowThrust(lt),
                )?;
                Ok(patch)
            }
            other => Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                field: "patch_type".to_string(),
                reason: format!("unknown trajectory patch type: {}", other),
            })),
        }
    }
}
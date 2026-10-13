use crate::ffi::{FfiVec3, OrbitPreview};
use rocketcon_sim::OrbitPreview as SessionPreview;

impl From<SessionPreview> for OrbitPreview {
    fn from(source: SessionPreview) -> Self {
        Self {
            reference_body_id: source.reference_body_id.to_string(),
            stellar_reference: source.stellar_reference,
            source_epoch_seconds: source.source_epoch_seconds,
            horizon_seconds: source.horizon_seconds,
            rendered_horizon_seconds: source.rendered_horizon_seconds,
            end_reason: source.end_reason as u8,
            has_impact: source.impact_epoch_seconds.is_some(),
            impact_epoch_seconds: source.impact_epoch_seconds.unwrap_or_default(),
            has_soi: source.soi.is_some(),
            soi_radius_m: source.soi.as_ref().map_or(0.0, |s| s.radius_m),
            soi_outside: source.soi.as_ref().is_some_and(|s| s.outside),
            has_soi_exit: source.soi.as_ref().is_some_and(|s| s.exit.is_some()),
            soi_exit_seconds: source.soi.as_ref().and_then(|s| s.exit.as_ref())
                .map_or(0.0, |event| event.time_to_seconds),
            soi_exit_position_m: source.soi.as_ref().and_then(|s| s.exit.as_ref())
                .map_or([0.0; 3], |event| event.relative_position_m).into(),
            under_thrust: source.under_thrust,
            has_inclination: source.nodes.inclination_rad.is_some(),
            inclination_rad: source.nodes.inclination_rad.unwrap_or_default(),
            has_ascending_longitude: source.nodes.ascending_longitude_rad.is_some(),
            ascending_longitude_rad: source.nodes.ascending_longitude_rad.unwrap_or_default(),
            has_next_ascending: source.nodes.next_ascending.is_some(),
            next_ascending_seconds: source.nodes.next_ascending.as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_ascending_position_m: source.nodes.next_ascending.as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m).into(),
            has_next_descending: source.nodes.next_descending.is_some(),
            next_descending_seconds: source.nodes.next_descending.as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_descending_position_m: source.nodes.next_descending.as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m).into(),
            has_orbit_view_basis: source.orbit_view_basis.is_some(),
            orbit_horizontal: source.orbit_view_basis.as_ref()
                .map_or([0.0; 3], |basis| basis.horizontal).into(),
            orbit_vertical: source.orbit_view_basis.as_ref()
                .map_or([0.0; 3], |basis| basis.vertical).into(),
            has_equatorial_reference: source.equatorial.is_some(),
            equatorial_normal: source.equatorial.as_ref()
                .map(|eq| eq.frame.as_arrays().0).unwrap_or([0.0; 3]).into(),
            equatorial_horizontal: source.equatorial.as_ref()
                .map(|eq| eq.frame.as_arrays().1).unwrap_or([0.0; 3]).into(),
            equatorial_vertical: source.equatorial.as_ref()
                .map(|eq| eq.frame.as_arrays().2).unwrap_or([0.0; 3]).into(),
            equatorial_has_inclination: source.equatorial.as_ref()
                .is_some_and(|eq| eq.nodes.inclination_rad.is_some()),
            equatorial_inclination_rad: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.inclination_rad).unwrap_or_default(),
            equatorial_has_ascending_longitude: source.equatorial.as_ref()
                .is_some_and(|eq| eq.nodes.ascending_longitude_rad.is_some()),
            equatorial_ascending_longitude_rad: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.ascending_longitude_rad).unwrap_or_default(),
            equatorial_has_next_ascending: source.equatorial.as_ref()
                .is_some_and(|eq| eq.nodes.next_ascending.is_some()),
            equatorial_next_ascending_seconds: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.next_ascending.as_ref())
                .map_or(0.0, |node| node.time_to_seconds),
            equatorial_next_ascending_position_m: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.next_ascending.as_ref())
                .map_or([0.0; 3], |node| node.relative_position_m).into(),
            equatorial_has_next_descending: source.equatorial.as_ref()
                .is_some_and(|eq| eq.nodes.next_descending.is_some()),
            equatorial_next_descending_seconds: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.next_descending.as_ref())
                .map_or(0.0, |node| node.time_to_seconds),
            equatorial_next_descending_position_m: source.equatorial.as_ref()
                .and_then(|eq| eq.nodes.next_descending.as_ref())
                .map_or([0.0; 3], |node| node.relative_position_m).into(),
            has_eccentricity: source.eccentricity.is_some(),
            eccentricity: source.eccentricity.unwrap_or_default(),
            has_periapsis: source.periapsis_altitude_m.is_some(),
            periapsis_altitude_m: source.periapsis_altitude_m.unwrap_or_default(),
            has_apoapsis: source.apoapsis_altitude_m.is_some(),
            apoapsis_altitude_m: source.apoapsis_altitude_m.unwrap_or_default(),
            has_period: source.period_seconds.is_some(),
            period_seconds: source.period_seconds.unwrap_or_default(),
            has_next_periapsis: source.next_periapsis.is_some(),
            next_periapsis_seconds: source
                .next_periapsis
                .as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_periapsis_position_m: source
                .next_periapsis
                .as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m)
                .into(),
            has_next_apoapsis: source.next_apoapsis.is_some(),
            next_apoapsis_seconds: source
                .next_apoapsis
                .as_ref()
                .map_or(0.0, |event| event.time_to_seconds),
            next_apoapsis_position_m: source
                .next_apoapsis
                .as_ref()
                .map_or([0.0; 3], |event| event.relative_position_m)
                .into(),
            relative_points_m: source
                .relative_points_m
                .into_iter()
                .map(FfiVec3::from)
                .collect(),
            osculating_closed_points_m: source
                .osculating_closed_points_m
                .into_iter()
                .map(FfiVec3::from)
                .collect(),
        }
    }
}

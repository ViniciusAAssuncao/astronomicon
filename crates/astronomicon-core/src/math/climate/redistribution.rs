use crate::math::climate::heat_capacity::combined_column_heat_capacity;

pub fn thermal_redistribution_efficiency(
    column_heat_capacity: f64,
    circulation_cells_per_hemisphere: u32,
) -> f64 {
    if column_heat_capacity <= 0.0 || !column_heat_capacity.is_finite() {
        return 0.0;
    }

    let n_cells = circulation_cells_per_hemisphere.max(1) as f64;
    let ref_heat_capacity = 2.5e6;
    let mass_buffering = column_heat_capacity / (column_heat_capacity + ref_heat_capacity);

    (mass_buffering / n_cells).clamp(0.0, 1.0)
}

pub fn combined_thermal_redistribution_efficiency(
    atmospheric_heat_capacity: f64,
    oceanic_heat_capacity: f64,
    ocean_coverage_fraction: f64,
    circulation_cells_per_hemisphere: u32,
) -> f64 {
    let combined_heat_capacity = combined_column_heat_capacity(
        atmospheric_heat_capacity,
        oceanic_heat_capacity,
        ocean_coverage_fraction,
    );

    if combined_heat_capacity <= 0.0 || !combined_heat_capacity.is_finite() {
        return 0.0;
    }

    let n_cells = circulation_cells_per_hemisphere.max(1) as f64;
    let ref_heat_capacity = 2.5e6;
    let ocean_cov = ocean_coverage_fraction.clamp(0.0, 1.0);
    let effective_cells = n_cells * (1.0 - 0.7 * ocean_cov);
    let mass_buffering = combined_heat_capacity / (combined_heat_capacity + ref_heat_capacity);

    (mass_buffering / effective_cells.max(1.0)).clamp(0.0, 1.0)
}

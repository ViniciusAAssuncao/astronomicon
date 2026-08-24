use crate::chemistry::solvent::SolventProperties;
use crate::math::thermodynamics::{
    mixing_ratio_from_relative_humidity, saturation_mixing_ratio, saturation_vapor_pressure,
};
use crate::units::{Length, MolarMass, Pressure, Temperature};

pub fn relative_humidity_at_altitude(
    surface_relative_humidity: f64,
    altitude: Length,
    tropopause_altitude: Length,
) -> f64 {
    let rh = surface_relative_humidity.clamp(0.0, 1.0);
    let z = altitude.value();
    let z_tropo = tropopause_altitude.value();

    if !rh.is_finite() || rh <= 0.0 || !z.is_finite() || z < 0.0 {
        return 0.0;
    }

    if z_tropo.is_finite() && z_tropo > 0.0 && z > z_tropo {
        return 0.0;
    }

    rh
}

pub fn mixing_ratio_at_altitude(
    relative_humidity: f64,
    temperature: Temperature,
    pressure: Pressure,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> f64 {
    let p_sat = saturation_vapor_pressure(temperature, solvent_properties);
    let r_sat = saturation_mixing_ratio(
        p_sat,
        pressure,
        solvent_molar_mass,
        atmospheric_molar_mass,
    );
    mixing_ratio_from_relative_humidity(relative_humidity, r_sat)
}
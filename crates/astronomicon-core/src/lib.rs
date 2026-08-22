pub mod chemistry;
pub mod domain;
pub mod error;
pub mod math;
pub mod units;

pub use chemistry::SolventProperties;
pub use domain::{
    Atmosphere, Barycenter, BarycenterMember, GasComponent, Hydrosphere, HydrosphereComponent,
    OrbitalElements, OrbitalParent, Planet, PlanetBuilder, PlanetKind, Star, StarBuilder,
    StarKind, StarSystem, TectonicRegime, UniverseState,
};
pub use math::{
    convective_to_yield_stress_ratio, crust_thermal_conductivity, determine_tectonic_regime,
    lithosphere_thickness, lithosphere_thickness_for_planet, lithosphere_yield_strength,
    mantle_convective_stress, mantle_solidus_temperature, plate_rms_velocity,
    tectonic_plate_count, HydrosphereStructure, MatterState, ResonanceState,
    SecularPrecessionRates,
};
pub use units::{
    Acceleration, AccelerationVector, Angle, AngularVelocity, Density, Duration, Energy, Frequency,
    GravitationalParameter, HeatFlux, Irradiance, Length, Luminosity, MagneticDipoleMoment,
    MagneticFluxDensity, MagneticRigidity, Mass, MassAttenuationCoefficient, MassRate, MolarMass,
    Position, Pressure, RadiationDose, Speed, Temperature, TemperatureGradient, Vector3, Velocity,
    Wavelength,
};

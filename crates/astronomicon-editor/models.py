from dataclasses import dataclass
from typing import Optional, List

STAR_KINDS: List[str] = [
    "Star",
    "WhiteDwarf",
    "NeutronStar",
    "BlackHole",
    "BrownDwarf",
    "Exotic",
]

PLANET_KINDS: List[str] = [
    "Telluric",
    "GasGiant",
    "IceGiant",
    "DwarfPlanet",
    "Chthonian",
    "CarbonPlanet",
    "IcyBody",
    "Exotic",
]


@dataclass
class OrbitalElements:
    semi_major_axis_m: float
    eccentricity: float
    inclination_rad: float
    longitude_ascending_node_rad: float
    argument_periapsis_rad: float
    mean_anomaly_at_epoch_rad: float


@dataclass
class StarSystem:
    id: str
    name: str
    right_ascension_rad: Optional[float] = None
    declination_rad: Optional[float] = None
    distance_from_sol_m: Optional[float] = None


@dataclass
class Star:
    id: str
    name: str
    kind: str
    mass_kg: float
    star_system_id: Optional[str] = None
    parent_star_id: Optional[str] = None
    parent_planet_id: Optional[str] = None
    parent_barycenter_id: Optional[str] = None
    radius_m: Optional[float] = None
    effective_temperature_k: Optional[float] = None
    rotation_period_s: Optional[float] = None
    axial_tilt_rad: Optional[float] = None
    semi_major_axis_m: Optional[float] = None
    eccentricity: Optional[float] = None
    inclination_rad: Optional[float] = None
    longitude_ascending_node_rad: Optional[float] = None
    argument_periapsis_rad: Optional[float] = None
    mean_anomaly_at_epoch_rad: Optional[float] = None
    oblateness_j2: Optional[float] = None


@dataclass
class Planet:
    id: str
    name: str
    kind: str
    mass_kg: float
    star_system_id: Optional[str] = None
    parent_star_id: Optional[str] = None
    parent_planet_id: Optional[str] = None
    parent_barycenter_id: Optional[str] = None
    equatorial_radius_m: Optional[float] = None
    polar_radius_m: Optional[float] = None
    rotation_period_s: Optional[float] = None
    axial_tilt_rad: Optional[float] = None
    geometric_albedo: Optional[float] = None
    bond_albedo: Optional[float] = None
    thermal_inertia: Optional[float] = None
    solstice_true_anomaly_rad: Optional[float] = None
    semi_major_axis_m: Optional[float] = None
    eccentricity: Optional[float] = None
    inclination_rad: Optional[float] = None
    longitude_ascending_node_rad: Optional[float] = None
    argument_periapsis_rad: Optional[float] = None
    mean_anomaly_at_epoch_rad: Optional[float] = None
    oblateness_j2: Optional[float] = None


@dataclass
class Barycenter:
    id: str
    name: str
    internal_semi_major_axis_m: float
    internal_eccentricity: float
    internal_inclination_rad: float
    internal_longitude_ascending_node_rad: float
    internal_argument_periapsis_rad: float
    internal_mean_anomaly_at_epoch_rad: float
    star_system_id: Optional[str] = None
    primary_star_id: Optional[str] = None
    primary_planet_id: Optional[str] = None
    primary_barycenter_id: Optional[str] = None
    secondary_star_id: Optional[str] = None
    secondary_planet_id: Optional[str] = None
    secondary_barycenter_id: Optional[str] = None
    parent_star_id: Optional[str] = None
    parent_planet_id: Optional[str] = None
    parent_barycenter_id: Optional[str] = None
    external_semi_major_axis_m: Optional[float] = None
    external_eccentricity: Optional[float] = None
    external_inclination_rad: Optional[float] = None
    external_longitude_ascending_node_rad: Optional[float] = None
    external_argument_periapsis_rad: Optional[float] = None
    external_mean_anomaly_at_epoch_rad: Optional[float] = None


@dataclass
class Atmosphere:
    id: str
    planet_id: str
    pressure_pa: float
    greenhouse_effect_k: float
    lapse_rate_k_per_m: float


@dataclass
class AtmosphereGasComponent:
    atmosphere_id: str
    formula: str
    percentage: float


@dataclass
class UniverseState:
    id: int = 1
    seconds_since_j2000_epoch: float = 0.0

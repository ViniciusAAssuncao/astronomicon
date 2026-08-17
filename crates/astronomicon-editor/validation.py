import math
from typing import List, Optional, Tuple
from elements import parse_molecular_formula
from models import (
    Atmosphere,
    AtmosphereGasComponent,
    Barycenter,
    OrbitalElements,
    Planet,
    PLANET_KINDS,
    Star,
    STAR_KINDS,
    StarSystem,
    UniverseState,
)

MAX_ATMOSPHERE_PERCENTAGE_OVERAGE: float = 0.001

def validate_molecular_formula(formula: str) -> List[str]:
    errors: List[str] = []
    _, err = parse_molecular_formula(formula)
    if err:
        errors.append(err)
    return errors

def validate_gas_component(component: AtmosphereGasComponent) -> List[str]:
    errors: List[str] = []
    errors.extend(validate_molecular_formula(component.formula))

    if not math.isfinite(component.percentage):
        errors.append("gas percentage must be a finite number")
    elif component.percentage < 0.0 or component.percentage > 100.0:
        errors.append("gas percentage must be between 0.0 and 100.0")

    return errors

def validate_gas_composition(components: List[AtmosphereGasComponent]) -> List[str]:
    errors: List[str] = []
    seen_formulas = set()
    total_percentage = 0.0

    for comp in components:
        errors.extend(validate_gas_component(comp))
        if comp.formula in seen_formulas:
            errors.append(f"duplicate gas formula '{comp.formula}' in atmosphere composition")
        else:
            seen_formulas.add(comp.formula)

        if math.isfinite(comp.percentage):
            total_percentage += comp.percentage

    if total_percentage > 100.0 + MAX_ATMOSPHERE_PERCENTAGE_OVERAGE:
        errors.append(
            f"total atmospheric gas percentage ({total_percentage:.4f}%) exceeds 100.0%"
        )

    return errors

def validate_atmosphere(
    atmosphere: Atmosphere,
    components: Optional[List[AtmosphereGasComponent]] = None,
) -> List[str]:
    errors: List[str] = []

    if not atmosphere.id or not atmosphere.id.strip():
        errors.append("atmosphere id cannot be empty")

    if not atmosphere.planet_id or not atmosphere.planet_id.strip():
        errors.append("atmosphere planet_id cannot be empty")

    if not math.isfinite(atmosphere.pressure_pa):
        errors.append("surface pressure must be a finite number")
    elif atmosphere.pressure_pa < 0.0:
        errors.append("surface pressure must be non-negative")

    if not math.isfinite(atmosphere.greenhouse_effect_k):
        errors.append("greenhouse effect must be a finite number")

    if not math.isfinite(atmosphere.lapse_rate_k_per_m):
        errors.append("lapse rate must be a finite number")

    if components is not None:
        errors.extend(validate_gas_composition(components))

    return errors

def validate_orbital_elements(elements: OrbitalElements, prefix: str = "") -> List[str]:
    errors: List[str] = []

    if not math.isfinite(elements.semi_major_axis_m):
        errors.append(f"{prefix}semi-major axis must be a finite number")
    elif elements.semi_major_axis_m <= 0.0:
        errors.append(f"{prefix}semi-major axis must be strictly positive")

    if not math.isfinite(elements.eccentricity):
        errors.append(f"{prefix}eccentricity must be a finite number")
    elif elements.eccentricity < 0.0 or elements.eccentricity >= 1.0:
        errors.append(f"{prefix}eccentricity must be in range [0.0, 1.0)")

    if not math.isfinite(elements.inclination_rad):
        errors.append(f"{prefix}inclination must be a finite number")

    if not math.isfinite(elements.longitude_ascending_node_rad):
        errors.append(f"{prefix}longitude of ascending node must be a finite number")

    if not math.isfinite(elements.argument_periapsis_rad):
        errors.append(f"{prefix}argument of periapsis must be a finite number")

    if not math.isfinite(elements.mean_anomaly_at_epoch_rad):
        errors.append(f"{prefix}mean anomaly at epoch must be a finite number")

    return errors

def _validate_orbital_hierarchy_and_elements(
    parent_star_id: Optional[str],
    parent_planet_id: Optional[str],
    parent_barycenter_id: Optional[str],
    semi_major_axis_m: Optional[float],
    eccentricity: Optional[float],
    inclination_rad: Optional[float],
    longitude_ascending_node_rad: Optional[float],
    argument_periapsis_rad: Optional[float],
    mean_anomaly_at_epoch_rad: Optional[float],
    prefix: str = "",
) -> List[str]:
    errors: List[str] = []

    parent_ids = [
        pid for pid in (parent_star_id, parent_planet_id, parent_barycenter_id)
        if pid is not None and pid.strip() != ""
    ]

    if len(parent_ids) > 1:
        errors.append(f"{prefix}at most one orbital parent can be specified")

    is_fixed = len(parent_ids) == 0

    orbital_values = [
        semi_major_axis_m,
        eccentricity,
        inclination_rad,
        longitude_ascending_node_rad,
        argument_periapsis_rad,
        mean_anomaly_at_epoch_rad,
    ]

    if is_fixed:
        if any(v is not None for v in orbital_values):
            errors.append(f"{prefix}fixed entity cannot have orbital elements defined")
    else:
        if any(v is None for v in orbital_values):
            errors.append(f"{prefix}orbiting entity must have all 6 orbital elements defined")
        else:
            elements = OrbitalElements(
                semi_major_axis_m=semi_major_axis_m,
                eccentricity=eccentricity,
                inclination_rad=inclination_rad,
                longitude_ascending_node_rad=longitude_ascending_node_rad,
                argument_periapsis_rad=argument_periapsis_rad,
                mean_anomaly_at_epoch_rad=mean_anomaly_at_epoch_rad,
            )
            errors.extend(validate_orbital_elements(elements, prefix=prefix))

    return errors

def validate_star_system(system: StarSystem) -> List[str]:
    errors: List[str] = []

    if not system.id or not system.id.strip():
        errors.append("star system id cannot be empty")

    if not system.name or not system.name.strip():
        errors.append("star system name cannot be empty")

    if system.distance_from_sol_m is not None:
        if not math.isfinite(system.distance_from_sol_m):
            errors.append("distance from sol must be a finite number")
        elif system.distance_from_sol_m <= 0.0:
            errors.append("distance from sol must be strictly positive")

    if system.right_ascension_rad is not None:
        if not math.isfinite(system.right_ascension_rad):
            errors.append("right ascension must be a finite number")

    if system.declination_rad is not None:
        if not math.isfinite(system.declination_rad):
            errors.append("declination must be a finite number")

    return errors

def validate_star(star: Star) -> List[str]:
    errors: List[str] = []

    if not star.id or not star.id.strip():
        errors.append("star id cannot be empty")

    if not star.name or not star.name.strip():
        errors.append("star name cannot be empty")

    if star.kind not in STAR_KINDS:
        errors.append(f"invalid star kind '{star.kind}'")

    if not math.isfinite(star.mass_kg):
        errors.append("star mass must be a finite number")
    elif star.mass_kg <= 0.0:
        errors.append("star mass must be strictly positive")

    if star.radius_m is not None:
        if not math.isfinite(star.radius_m):
            errors.append("star radius must be a finite number")
        elif star.radius_m <= 0.0:
            errors.append("star radius must be strictly positive")

    if star.effective_temperature_k is not None:
        if not math.isfinite(star.effective_temperature_k):
            errors.append("effective temperature must be a finite number")
        elif star.effective_temperature_k <= 0.0:
            errors.append("effective temperature must be strictly positive")

    if star.rotation_period_s is not None:
        if not math.isfinite(star.rotation_period_s):
            errors.append("rotation period must be a finite number")
        elif star.rotation_period_s <= 0.0:
            errors.append("rotation period must be strictly positive")

    if star.axial_tilt_rad is not None:
        if not math.isfinite(star.axial_tilt_rad):
            errors.append("axial tilt must be a finite number")

    if star.oblateness_j2 is not None:
        if not math.isfinite(star.oblateness_j2):
            errors.append("oblateness j2 must be a finite number")

    errors.extend(
        _validate_orbital_hierarchy_and_elements(
            star.parent_star_id,
            star.parent_planet_id,
            star.parent_barycenter_id,
            star.semi_major_axis_m,
            star.eccentricity,
            star.inclination_rad,
            star.longitude_ascending_node_rad,
            star.argument_periapsis_rad,
            star.mean_anomaly_at_epoch_rad,
        )
    )

    return errors

def validate_planet(planet: Planet) -> List[str]:
    errors: List[str] = []

    if not planet.id or not planet.id.strip():
        errors.append("planet id cannot be empty")

    if not planet.name or not planet.name.strip():
        errors.append("planet name cannot be empty")

    if planet.kind not in PLANET_KINDS:
        errors.append(f"invalid planet kind '{planet.kind}'")

    if not math.isfinite(planet.mass_kg):
        errors.append("planet mass must be a finite number")
    elif planet.mass_kg <= 0.0:
        errors.append("planet mass must be strictly positive")

    if planet.equatorial_radius_m is not None:
        if not math.isfinite(planet.equatorial_radius_m):
            errors.append("equatorial radius must be a finite number")
        elif planet.equatorial_radius_m <= 0.0:
            errors.append("equatorial radius must be strictly positive")

    if planet.polar_radius_m is not None:
        if not math.isfinite(planet.polar_radius_m):
            errors.append("polar radius must be a finite number")
        elif planet.polar_radius_m <= 0.0:
            errors.append("polar radius must be strictly positive")

    if planet.rotation_period_s is not None:
        if not math.isfinite(planet.rotation_period_s):
            errors.append("rotation period must be a finite number")
        elif planet.rotation_period_s <= 0.0:
            errors.append("rotation period must be strictly positive")

    if planet.axial_tilt_rad is not None:
        if not math.isfinite(planet.axial_tilt_rad):
            errors.append("axial tilt must be a finite number")

    if planet.geometric_albedo is not None:
        if not math.isfinite(planet.geometric_albedo):
            errors.append("geometric albedo must be a finite number")
        elif planet.geometric_albedo < 0.0 or planet.geometric_albedo > 1.0:
            errors.append("geometric albedo must be between 0.0 and 1.0")

    if planet.bond_albedo is not None:
        if not math.isfinite(planet.bond_albedo):
            errors.append("bond albedo must be a finite number")
        elif planet.bond_albedo < 0.0 or planet.bond_albedo > 1.0:
            errors.append("bond albedo must be between 0.0 and 1.0")

    if planet.thermal_inertia is not None:
        if not math.isfinite(planet.thermal_inertia):
            errors.append("thermal inertia must be a finite number")
        elif planet.thermal_inertia < 0.0 or planet.thermal_inertia > 1.0:
            errors.append("thermal inertia must be between 0.0 and 1.0")

    if planet.solstice_true_anomaly_rad is not None:
        if not math.isfinite(planet.solstice_true_anomaly_rad):
            errors.append("solstice true anomaly must be a finite number")

    if planet.oblateness_j2 is not None:
        if not math.isfinite(planet.oblateness_j2):
            errors.append("oblateness j2 must be a finite number")

    errors.extend(
        _validate_orbital_hierarchy_and_elements(
            planet.parent_star_id,
            planet.parent_planet_id,
            planet.parent_barycenter_id,
            planet.semi_major_axis_m,
            planet.eccentricity,
            planet.inclination_rad,
            planet.longitude_ascending_node_rad,
            planet.argument_periapsis_rad,
            planet.mean_anomaly_at_epoch_rad,
        )
    )

    return errors

def validate_barycenter(barycenter: Barycenter) -> List[str]:
    errors: List[str] = []

    if not barycenter.id or not barycenter.id.strip():
        errors.append("barycenter id cannot be empty")

    if not barycenter.name or not barycenter.name.strip():
        errors.append("barycenter name cannot be empty")

    primary_slots = [
        pid for pid in (
            barycenter.primary_star_id,
            barycenter.primary_planet_id,
            barycenter.primary_barycenter_id,
        )
        if pid is not None and pid.strip() != ""
    ]

    if len(primary_slots) != 1:
        errors.append("primary member must specify exactly one entity reference")
        primary_id = None
    else:
        primary_id = primary_slots[0]

    secondary_slots = [
        pid for pid in (
            barycenter.secondary_star_id,
            barycenter.secondary_planet_id,
            barycenter.secondary_barycenter_id,
        )
        if pid is not None and pid.strip() != ""
    ]

    if len(secondary_slots) != 1:
        errors.append("secondary member must specify exactly one entity reference")
        secondary_id = None
    else:
        secondary_id = secondary_slots[0]

    if primary_id is not None and secondary_id is not None:
        if primary_id == secondary_id:
            errors.append("primary and secondary members must be distinct entities")

    if barycenter.id:
        if primary_id == barycenter.id or secondary_id == barycenter.id:
            errors.append("barycenter cannot be a member of itself")

    internal_elements = OrbitalElements(
        semi_major_axis_m=barycenter.internal_semi_major_axis_m,
        eccentricity=barycenter.internal_eccentricity,
        inclination_rad=barycenter.internal_inclination_rad,
        longitude_ascending_node_rad=barycenter.internal_longitude_ascending_node_rad,
        argument_periapsis_rad=barycenter.internal_argument_periapsis_rad,
        mean_anomaly_at_epoch_rad=barycenter.internal_mean_anomaly_at_epoch_rad,
    )
    errors.extend(validate_orbital_elements(internal_elements, prefix="internal "))

    if barycenter.parent_barycenter_id and barycenter.id:
        if barycenter.parent_barycenter_id == barycenter.id:
            errors.append("barycenter cannot have itself as orbital parent")

    errors.extend(
        _validate_orbital_hierarchy_and_elements(
            barycenter.parent_star_id,
            barycenter.parent_planet_id,
            barycenter.parent_barycenter_id,
            barycenter.external_semi_major_axis_m,
            barycenter.external_eccentricity,
            barycenter.external_inclination_rad,
            barycenter.external_longitude_ascending_node_rad,
            barycenter.external_argument_periapsis_rad,
            barycenter.external_mean_anomaly_at_epoch_rad,
            prefix="external ",
        )
    )

    return errors

def validate_universe_state(state: UniverseState) -> List[str]:
    errors: List[str] = []

    if state.id != 1:
        errors.append("universe state id must be 1")

    if not math.isfinite(state.seconds_since_j2000_epoch):
        errors.append("seconds since J2000 epoch must be a finite number")

    return errors

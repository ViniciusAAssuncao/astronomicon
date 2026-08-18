import math
from typing import Tuple

GRAVITATIONAL_CONSTANT: float = 6.67430e-11
SPEED_OF_LIGHT: float = 299792458.0
STEFAN_BOLTZMANN_CONSTANT: float = 5.670374419e-8
UNIVERSAL_GAS_CONSTANT: float = 8.314462618
ASTRONOMICAL_UNIT: float = 149597870700.0
LIGHT_YEAR: float = 9460730472580800.0
PARSEC: float = 3.085677581491367e16

SOLAR_MASS: float = 1.98847e30
SOLAR_RADIUS: float = 6.957e8
SOLAR_TEMPERATURE: float = 5778.0
SOLAR_LUMINOSITY: float = 3.828e26

EARTH_MASS: float = 5.9722e24
EARTH_RADIUS: float = 6.371e6
EARTH_EQUATORIAL_RADIUS: float = 6.378137e6
JUPITER_MASS: float = 1.89813e27

ROCHE_FLUID_COEFFICIENT: float = 2.44
MARDLING_AARSETH_CRITICAL_COEFFICIENT: float = 2.8
MARDLING_AARSETH_MASS_EXPONENT: float = 0.4
MARDLING_AARSETH_INCLINATION_COEFFICIENT: float = 0.33


def gravitational_parameter(mass: float) -> float:
    if mass <= 0.0 or not math.isfinite(mass):
        return 0.0
    return GRAVITATIONAL_CONSTANT * mass


def surface_gravity(mass: float, radius: float) -> float:
    if mass <= 0.0 or radius <= 0.0 or not math.isfinite(mass) or not math.isfinite(radius):
        return 0.0
    mu = gravitational_parameter(mass)
    return mu / (radius * radius)


def surface_gravity_from_mu(mu: float, radius: float) -> float:
    if mu <= 0.0 or radius <= 0.0 or not math.isfinite(mu) or not math.isfinite(radius):
        return 0.0
    return mu / (radius * radius)


def mean_density(mass: float, radius: float) -> float:
    if mass <= 0.0 or radius <= 0.0 or not math.isfinite(mass) or not math.isfinite(radius):
        return 0.0
    volume = (4.0 / 3.0) * math.pi * (radius ** 3)
    return mass / volume


def stellar_luminosity(radius: float, temperature: float) -> float:
    if radius <= 0.0 or temperature <= 0.0 or not math.isfinite(radius) or not math.isfinite(temperature):
        return 0.0
    area = 4.0 * math.pi * radius * radius
    t4 = temperature ** 4
    return area * STEFAN_BOLTZMANN_CONSTANT * t4


def orbital_irradiance(luminosity: float, distance: float) -> float:
    if luminosity <= 0.0 or distance <= 0.0 or not math.isfinite(luminosity) or not math.isfinite(distance):
        return 0.0
    area = 4.0 * math.pi * distance * distance
    return luminosity / area


def equilibrium_temperature(
    star_temperature: float,
    star_radius: float,
    orbital_distance: float,
    bond_albedo: float = 0.0,
) -> float:
    if (
        star_temperature <= 0.0
        or star_radius <= 0.0
        or orbital_distance <= 0.0
        or not math.isfinite(star_temperature)
        or not math.isfinite(star_radius)
        or not math.isfinite(orbital_distance)
    ):
        return 0.0
    albedo = max(
        0.0, min(1.0, bond_albedo if math.isfinite(bond_albedo) else 0.0))
    lum = stellar_luminosity(star_radius, star_temperature)
    irr = orbital_irradiance(lum, orbital_distance)
    absorbed = (1.0 - albedo) * irr
    t4 = absorbed / (4.0 * STEFAN_BOLTZMANN_CONSTANT)
    return max(0.0, t4) ** 0.25


def roche_limit_rigid(
    primary_radius: float,
    primary_density: float,
    satellite_density: float,
) -> float:
    if (
        primary_radius <= 0.0
        or primary_density <= 0.0
        or satellite_density <= 0.0
        or not math.isfinite(primary_radius)
        or not math.isfinite(primary_density)
        or not math.isfinite(satellite_density)
    ):
        return 0.0
    ratio = 2.0 * primary_density / satellite_density
    return primary_radius * (ratio ** (1.0 / 3.0))


def roche_limit_fluid(
    primary_radius: float,
    primary_density: float,
    satellite_density: float,
) -> float:
    if (
        primary_radius <= 0.0
        or primary_density <= 0.0
        or satellite_density <= 0.0
        or not math.isfinite(primary_radius)
        or not math.isfinite(primary_density)
        or not math.isfinite(satellite_density)
    ):
        return 0.0
    ratio = primary_density / satellite_density
    return ROCHE_FLUID_COEFFICIENT * primary_radius * (ratio ** (1.0 / 3.0))


def hill_sphere_radius(
    semi_major_axis: float,
    body_mass: float,
    parent_mass: float,
    eccentricity: float = 0.0,
) -> float:
    if (
        semi_major_axis <= 0.0
        or body_mass <= 0.0
        or parent_mass <= 0.0
        or not math.isfinite(semi_major_axis)
        or not math.isfinite(body_mass)
        or not math.isfinite(parent_mass)
    ):
        return 0.0
    e = max(0.0, min(1.0, eccentricity if math.isfinite(eccentricity) else 0.0))
    periapsis = semi_major_axis * (1.0 - e)
    mass_ratio = body_mass / (3.0 * parent_mass)
    return periapsis * (mass_ratio ** (1.0 / 3.0))


def habitable_zone_boundaries(luminosity: float) -> Tuple[float, float]:
    if luminosity <= 0.0 or not math.isfinite(luminosity):
        return 0.0, 0.0
    relative_lum = luminosity / SOLAR_LUMINOSITY
    sqrt_l = math.sqrt(relative_lum)
    inner_boundary = 0.95 * ASTRONOMICAL_UNIT * sqrt_l
    outer_boundary = 1.37 * ASTRONOMICAL_UNIT * sqrt_l
    return inner_boundary, outer_boundary


def orbital_period(semi_major_axis: float, mu: float) -> float:
    if semi_major_axis <= 0.0 or mu <= 0.0 or not math.isfinite(semi_major_axis) or not math.isfinite(mu):
        return 0.0
    return 2.0 * math.pi * math.sqrt((semi_major_axis ** 3) / mu)


def mean_motion(semi_major_axis: float, mu: float) -> float:
    if semi_major_axis <= 0.0 or mu <= 0.0 or not math.isfinite(semi_major_axis) or not math.isfinite(mu):
        return 0.0
    return math.sqrt(mu / (semi_major_axis ** 3))


def orbital_speed(mu: float, radius: float, semi_major_axis: float) -> float:
    if (
        mu <= 0.0
        or radius <= 0.0
        or semi_major_axis <= 0.0
        or not math.isfinite(mu)
        or not math.isfinite(radius)
        or not math.isfinite(semi_major_axis)
    ):
        return 0.0
    v_sq = mu * (2.0 / radius - 1.0 / semi_major_axis)
    return math.sqrt(max(0.0, v_sq))


def escape_velocity(mass: float, radius: float) -> float:
    if mass <= 0.0 or radius <= 0.0 or not math.isfinite(mass) or not math.isfinite(radius):
        return 0.0
    mu = gravitational_parameter(mass)
    return math.sqrt(2.0 * mu / radius)


def escape_velocity_from_mu(mu: float, radius: float) -> float:
    if mu <= 0.0 or radius <= 0.0 or not math.isfinite(mu) or not math.isfinite(radius):
        return 0.0
    return math.sqrt(2.0 * mu / radius)


def schwarzschild_radius(mass: float) -> float:
    if mass <= 0.0 or not math.isfinite(mass):
        return 0.0
    return (2.0 * GRAVITATIONAL_CONSTANT * mass) / (SPEED_OF_LIGHT * SPEED_OF_LIGHT)


def mardling_aarseth_critical_ratio(
    inner_mass: float,
    outer_mass: float,
    outer_eccentricity: float = 0.0,
    mutual_inclination_rad: float = 0.0,
) -> float:
    if inner_mass <= 0.0 or outer_mass <= 0.0 or not math.isfinite(inner_mass) or not math.isfinite(outer_mass):
        return 0.0
    q = outer_mass / inner_mass
    e_out = max(0.0, min(0.9999, outer_eccentricity if math.isfinite(outer_eccentricity) else 0.0))
    denom = math.sqrt(max(1e-6, 1.0 - e_out))
    bracket = (((1.0 + q) * (1.0 + e_out)) / denom) ** MARDLING_AARSETH_MASS_EXPONENT
    inc_norm = (mutual_inclination_rad % math.pi) / math.pi if math.isfinite(mutual_inclination_rad) else 0.0
    inc_term = 1.0 - MARDLING_AARSETH_INCLINATION_COEFFICIENT * inc_norm
    return MARDLING_AARSETH_CRITICAL_COEFFICIENT * bracket * inc_term


def mardling_aarseth_stability_ratio(
    inner_semi_major_axis: float,
    outer_periapsis: float,
) -> float:
    if inner_semi_major_axis <= 0.0 or not math.isfinite(inner_semi_major_axis) or outer_periapsis <= 0.0:
        return 0.0
    return outer_periapsis / inner_semi_major_axis


def is_hierarchically_stable(
    inner_semi_major_axis: float,
    outer_periapsis: float,
    inner_mass: float,
    outer_mass: float,
    outer_eccentricity: float = 0.0,
    mutual_inclination_rad: float = 0.0,
) -> bool:
    actual = mardling_aarseth_stability_ratio(inner_semi_major_axis, outer_periapsis)
    critical = mardling_aarseth_critical_ratio(
        inner_mass, outer_mass, outer_eccentricity, mutual_inclination_rad
    )
    return actual >= critical

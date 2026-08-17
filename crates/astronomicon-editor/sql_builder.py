import uuid
from typing import Any, List, Optional
from models import (
    Atmosphere,
    AtmosphereGasComponent,
    Barycenter,
    Planet,
    Star,
    StarSystem,
    UniverseState,
)


def generate_uuid() -> str:
    return str(uuid.uuid4()).lower()


def sql_format_value(val: Any) -> str:
    if val is None:
        return "NULL"
    if isinstance(val, str):
        escaped = val.replace("'", "''")
        return f"'{escaped}'"
    if isinstance(val, bool):
        return "1" if val else "0"
    if isinstance(val, (int, float)):
        return repr(val)
    return f"'{str(val).replace("'", "''")}'"


def wrap_transaction(sql_body: str, atomic: bool = False) -> str:
    if not atomic:
        return sql_body.strip() + "\n"
    return f"BEGIN TRANSACTION;\n{sql_body.strip()}\nCOMMIT;\n"


def build_insert_star_system(system: StarSystem, atomic: bool = False) -> str:
    sys_id = system.id if system.id and system.id.strip() else generate_uuid()

    cols = [
        "id",
        "name",
        "right_ascension_rad",
        "declination_rad",
        "distance_from_sol_m",
    ]
    vals = [
        sql_format_value(sys_id),
        sql_format_value(system.name),
        sql_format_value(system.right_ascension_rad),
        sql_format_value(system.declination_rad),
        sql_format_value(system.distance_from_sol_m),
    ]

    sql = (
        f"INSERT INTO star_systems ({', '.join(cols)})\n"
        f"VALUES ({', '.join(vals)});"
    )
    return wrap_transaction(sql, atomic)


def build_insert_star(star: Star, atomic: bool = False) -> str:
    s_id = star.id if star.id and star.id.strip() else generate_uuid()

    cols = [
        "id",
        "star_system_id",
        "parent_star_id",
        "parent_planet_id",
        "parent_barycenter_id",
        "name",
        "kind",
        "mass_kg",
        "radius_m",
        "effective_temperature_k",
        "rotation_period_s",
        "axial_tilt_rad",
        "semi_major_axis_m",
        "eccentricity",
        "inclination_rad",
        "longitude_ascending_node_rad",
        "argument_periapsis_rad",
        "mean_anomaly_at_epoch_rad",
        "oblateness_j2",
    ]
    vals = [
        sql_format_value(s_id),
        sql_format_value(star.star_system_id),
        sql_format_value(star.parent_star_id),
        sql_format_value(star.parent_planet_id),
        sql_format_value(star.parent_barycenter_id),
        sql_format_value(star.name),
        sql_format_value(star.kind),
        sql_format_value(star.mass_kg),
        sql_format_value(star.radius_m),
        sql_format_value(star.effective_temperature_k),
        sql_format_value(star.rotation_period_s),
        sql_format_value(star.axial_tilt_rad),
        sql_format_value(star.semi_major_axis_m),
        sql_format_value(star.eccentricity),
        sql_format_value(star.inclination_rad),
        sql_format_value(star.longitude_ascending_node_rad),
        sql_format_value(star.argument_periapsis_rad),
        sql_format_value(star.mean_anomaly_at_epoch_rad),
        sql_format_value(star.oblateness_j2),
    ]

    sql = (
        f"INSERT INTO stars ({', '.join(cols)})\n"
        f"VALUES ({', '.join(vals)});"
    )
    return wrap_transaction(sql, atomic)


def build_insert_planet(planet: Planet, atomic: bool = False) -> str:
    p_id = planet.id if planet.id and planet.id.strip() else generate_uuid()

    cols = [
        "id",
        "star_system_id",
        "parent_star_id",
        "parent_planet_id",
        "parent_barycenter_id",
        "name",
        "kind",
        "mass_kg",
        "equatorial_radius_m",
        "polar_radius_m",
        "rotation_period_s",
        "axial_tilt_rad",
        "geometric_albedo",
        "bond_albedo",
        "thermal_inertia",
        "solstice_true_anomaly_rad",
        "semi_major_axis_m",
        "eccentricity",
        "inclination_rad",
        "longitude_ascending_node_rad",
        "argument_periapsis_rad",
        "mean_anomaly_at_epoch_rad",
        "oblateness_j2",
    ]
    vals = [
        sql_format_value(p_id),
        sql_format_value(planet.star_system_id),
        sql_format_value(planet.parent_star_id),
        sql_format_value(planet.parent_planet_id),
        sql_format_value(planet.parent_barycenter_id),
        sql_format_value(planet.name),
        sql_format_value(planet.kind),
        sql_format_value(planet.mass_kg),
        sql_format_value(planet.equatorial_radius_m),
        sql_format_value(planet.polar_radius_m),
        sql_format_value(planet.rotation_period_s),
        sql_format_value(planet.axial_tilt_rad),
        sql_format_value(planet.geometric_albedo),
        sql_format_value(planet.bond_albedo),
        sql_format_value(planet.thermal_inertia),
        sql_format_value(planet.solstice_true_anomaly_rad),
        sql_format_value(planet.semi_major_axis_m),
        sql_format_value(planet.eccentricity),
        sql_format_value(planet.inclination_rad),
        sql_format_value(planet.longitude_ascending_node_rad),
        sql_format_value(planet.argument_periapsis_rad),
        sql_format_value(planet.mean_anomaly_at_epoch_rad),
        sql_format_value(planet.oblateness_j2),
    ]

    sql = (
        f"INSERT INTO planets ({', '.join(cols)})\n"
        f"VALUES ({', '.join(vals)});"
    )
    return wrap_transaction(sql, atomic)


def build_insert_barycenter(barycenter: Barycenter, atomic: bool = False) -> str:
    b_id = barycenter.id if barycenter.id and barycenter.id.strip() else generate_uuid()

    cols = [
        "id",
        "star_system_id",
        "name",
        "primary_star_id",
        "primary_planet_id",
        "primary_barycenter_id",
        "secondary_star_id",
        "secondary_planet_id",
        "secondary_barycenter_id",
        "internal_semi_major_axis_m",
        "internal_eccentricity",
        "internal_inclination_rad",
        "internal_longitude_ascending_node_rad",
        "internal_argument_periapsis_rad",
        "internal_mean_anomaly_at_epoch_rad",
        "parent_star_id",
        "parent_planet_id",
        "parent_barycenter_id",
        "external_semi_major_axis_m",
        "external_eccentricity",
        "external_inclination_rad",
        "external_longitude_ascending_node_rad",
        "external_argument_periapsis_rad",
        "external_mean_anomaly_at_epoch_rad",
    ]
    vals = [
        sql_format_value(b_id),
        sql_format_value(barycenter.star_system_id),
        sql_format_value(barycenter.name),
        sql_format_value(barycenter.primary_star_id),
        sql_format_value(barycenter.primary_planet_id),
        sql_format_value(barycenter.primary_barycenter_id),
        sql_format_value(barycenter.secondary_star_id),
        sql_format_value(barycenter.secondary_planet_id),
        sql_format_value(barycenter.secondary_barycenter_id),
        sql_format_value(barycenter.internal_semi_major_axis_m),
        sql_format_value(barycenter.internal_eccentricity),
        sql_format_value(barycenter.internal_inclination_rad),
        sql_format_value(barycenter.internal_longitude_ascending_node_rad),
        sql_format_value(barycenter.internal_argument_periapsis_rad),
        sql_format_value(barycenter.internal_mean_anomaly_at_epoch_rad),
        sql_format_value(barycenter.parent_star_id),
        sql_format_value(barycenter.parent_planet_id),
        sql_format_value(barycenter.parent_barycenter_id),
        sql_format_value(barycenter.external_semi_major_axis_m),
        sql_format_value(barycenter.external_eccentricity),
        sql_format_value(barycenter.external_inclination_rad),
        sql_format_value(barycenter.external_longitude_ascending_node_rad),
        sql_format_value(barycenter.external_argument_periapsis_rad),
        sql_format_value(barycenter.external_mean_anomaly_at_epoch_rad),
    ]

    sql = (
        f"INSERT INTO barycenters ({', '.join(cols)})\n"
        f"VALUES ({', '.join(vals)});"
    )
    return wrap_transaction(sql, atomic)


def build_insert_atmosphere(
    atmosphere: Atmosphere,
    components: Optional[List[AtmosphereGasComponent]] = None,
    atomic: bool = False,
) -> str:
    atm_id = atmosphere.id if atmosphere.id and atmosphere.id.strip() else generate_uuid()

    atm_cols = [
        "id",
        "planet_id",
        "pressure_pa",
        "greenhouse_effect_k",
        "lapse_rate_k_per_m",
    ]
    atm_vals = [
        sql_format_value(atm_id),
        sql_format_value(atmosphere.planet_id),
        sql_format_value(atmosphere.pressure_pa),
        sql_format_value(atmosphere.greenhouse_effect_k),
        sql_format_value(atmosphere.lapse_rate_k_per_m),
    ]

    statements = [
        f"INSERT INTO atmospheres ({', '.join(atm_cols)})\n"
        f"VALUES ({', '.join(atm_vals)});"
    ]

    if components:
        comp_cols = ["atmosphere_id", "formula", "percentage"]
        for comp in components:
            c_vals = [
                sql_format_value(atm_id),
                sql_format_value(comp.formula),
                sql_format_value(comp.percentage),
            ]
            statements.append(
                f"INSERT INTO atmosphere_gas_components ({', '.join(comp_cols)})\n"
                f"VALUES ({', '.join(c_vals)});"
            )

    sql = "\n".join(statements)
    return wrap_transaction(sql, atomic)


def build_insert_universe_state(state: UniverseState, atomic: bool = False) -> str:
    cols = ["id", "seconds_since_j2000_epoch"]
    vals = [
        sql_format_value(state.id),
        sql_format_value(state.seconds_since_j2000_epoch),
    ]
    sql = (
        f"INSERT INTO universe_state ({', '.join(cols)})\n"
        f"VALUES ({', '.join(vals)})\n"
        f"ON CONFLICT(id) DO UPDATE SET seconds_since_j2000_epoch = excluded.seconds_since_j2000_epoch;"
    )
    return wrap_transaction(sql, atomic)


def build_insert_sql(
    entity: Any,
    components: Optional[List[AtmosphereGasComponent]] = None,
    atomic: bool = True,
) -> str:
    if isinstance(entity, StarSystem):
        return build_insert_star_system(entity, atomic=atomic)
    if isinstance(entity, Star):
        return build_insert_star(entity, atomic=atomic)
    if isinstance(entity, Planet):
        return build_insert_planet(entity, atomic=atomic)
    if isinstance(entity, Barycenter):
        return build_insert_barycenter(entity, atomic=atomic)
    if isinstance(entity, Atmosphere):
        return build_insert_atmosphere(entity, components=components, atomic=atomic)
    if isinstance(entity, UniverseState):
        return build_insert_universe_state(entity, atomic=atomic)
    raise TypeError(f"unsupported entity type: {type(entity).__name__}")


if __name__ == "__main__":
    test_sys = StarSystem(
        id=generate_uuid(),
        name="Sirius' Prime System",
        right_ascension_rad=1.75,
        declination_rad=-0.45,
        distance_from_sol_m=8.14e16,
    )
    print("--- StarSystem SQL ---")
    print(build_insert_star_system(test_sys, atomic=True))

    test_star = Star(
        id=generate_uuid(),
        name="Sirius A's Core",
        kind="Star",
        mass_kg=4.1e30,
        star_system_id=test_sys.id,
        radius_m=1.19e9,
        effective_temperature_k=9940.0,
    )
    print("--- Star SQL ---")
    print(build_insert_star(test_star, atomic=False))

    test_planet = Planet(
        id=generate_uuid(),
        name="D'Khor II",
        kind="Telluric",
        mass_kg=5.972e24,
        star_system_id=test_sys.id,
        parent_star_id=test_star.id,
        equatorial_radius_m=6371000.0,
        semi_major_axis_m=1.495e11,
        eccentricity=0.016,
        inclination_rad=0.0,
        longitude_ascending_node_rad=0.0,
        argument_periapsis_rad=0.0,
        mean_anomaly_at_epoch_rad=0.0,
    )
    print("--- Planet SQL ---")
    print(build_insert_planet(test_planet, atomic=True))

    test_atm = Atmosphere(
        id=generate_uuid(),
        planet_id=test_planet.id,
        pressure_pa=101325.0,
        greenhouse_effect_k=33.0,
        lapse_rate_k_per_m=0.0065,
    )
    test_comps = [
        AtmosphereGasComponent(atmosphere_id=test_atm.id,
                               formula="N2", percentage=78.08),
        AtmosphereGasComponent(atmosphere_id=test_atm.id,
                               formula="O2", percentage=20.95),
        AtmosphereGasComponent(atmosphere_id=test_atm.id,
                               formula="CO2", percentage=0.04),
    ]
    print("--- Atmosphere SQL ---")
    print(build_insert_atmosphere(test_atm, components=test_comps, atomic=True))

import math
from typing import Dict, List, Optional

import cache
from models import (
    Atmosphere,
    AtmosphereGasComponent,
    Barycenter,
    Planet,
    Star,
    StarSystem,
)
import physics_lite
import reference_data
import units

ROUGH_MOLAR_MASS: Dict[str, float] = {
    "H": 1.008,
    "H2": 2.016,
    "HE": 4.003,
    "C": 12.011,
    "N": 14.007,
    "N2": 28.014,
    "O": 15.999,
    "O2": 31.998,
    "O3": 47.997,
    "F2": 37.996,
    "NE": 20.180,
    "NA": 22.990,
    "MG": 24.305,
    "SI": 28.085,
    "P": 30.974,
    "S": 32.060,
    "CL2": 70.900,
    "AR": 39.948,
    "CO": 28.010,
    "CO2": 44.009,
    "CH4": 16.043,
    "H2O": 18.015,
    "NH3": 17.031,
    "SO2": 64.066,
    "H2S": 34.080,
    "HCN": 27.026,
    "NO": 30.006,
    "NO2": 46.006,
    "N2O": 44.013,
    "KR": 83.798,
    "XE": 131.293,
    "FE": 55.845,
    "TI": 47.867,
    "TIO": 63.866,
    "VO": 66.941,
}


def curate_star(star: Star) -> List[str]:
    warnings: List[str] = []

    if star.mass_kg > 0.0 and star.radius_m is not None and star.radius_m > 0.0:
        dens = physics_lite.mean_density(star.mass_kg, star.radius_m)
        stats = reference_data.get_star_statistics(star.kind)
        dens_stat = stats.get("density_kg_per_m3") or stats.get("density")
        if dens_stat:
            min_d = dens_stat.get("min")
            max_d = dens_stat.get("max")
            if min_d is not None and dens < min_d * 0.1:
                warnings.append(
                    f"Densidade média ({dens:.2e} kg/m³) muito abaixo da faixa esperada para '{star.kind}' (mínimo típico ~{min_d:.2e} kg/m³)."
                )
            if max_d is not None and dens > max_d * 10.0:
                warnings.append(
                    f"Densidade média ({dens:.2e} kg/m³) muito acima da faixa esperada para '{star.kind}' (máximo típico ~{max_d:.2e} kg/m³)."
                )

    if (
        star.mass_kg > 0.0
        and star.radius_m is not None
        and star.radius_m > 0.0
        and star.rotation_period_s is not None
        and star.rotation_period_s > 0.0
    ):
        omega = (2.0 * math.pi) / star.rotation_period_s
        v_eq = omega * star.radius_m
        v_break = math.sqrt((physics_lite.GRAVITATIONAL_CONSTANT * star.mass_kg) / star.radius_m)
        if v_eq >= v_break:
            warnings.append(
                f"Velocidade equatorial de rotação ({v_eq:.1f} m/s) excede a velocidade de ruptura gravitacional ({v_break:.1f} m/s)."
            )
        elif v_eq >= 0.7 * v_break:
            warnings.append(
                f"Velocidade equatorial de rotação ({v_eq:.1f} m/s) atinge {(v_eq / v_break) * 100:.1f}% da velocidade de ruptura ({v_break:.1f} m/s)."
            )

    if star.kind == "WhiteDwarf":
        if star.radius_m is not None and star.radius_m > 5.0e7:
            warnings.append("Raio excessivamente grande para uma anã branca (> 50.000 km).")
        if star.mass_kg > 1.44 * units.SOLAR_MASS_KG:
            warnings.append(
                f"Massa ({star.mass_kg / units.SOLAR_MASS_KG:.2f} M☉) excede o limite de Chandrasekhar (~1.44 M☉)."
            )

    elif star.kind == "NeutronStar":
        if star.radius_m is not None and star.radius_m > 5.0e4:
            warnings.append("Raio excessivamente grande para uma estrela de nêutrons (> 50 km).")
        if star.mass_kg > 3.0 * units.SOLAR_MASS_KG:
            warnings.append(
                f"Massa ({star.mass_kg / units.SOLAR_MASS_KG:.2f} M☉) excede o limite de Tolman-Oppenheimer-Volkoff (~3.0 M☉)."
            )

    elif star.kind == "BlackHole":
        if star.radius_m is not None and star.mass_kg > 0.0:
            r_s = physics_lite.schwarzschild_radius(star.mass_kg)
            if star.radius_m > 5.0 * r_s:
                warnings.append(
                    f"Raio físico informado ({star.radius_m:.1f} m) é substancialmente maior que o raio de Schwarzschild ({r_s:.1f} m)."
                )

    elif star.kind == "BrownDwarf":
        if star.mass_kg > 0.08 * units.SOLAR_MASS_KG:
            warnings.append(
                f"Massa ({star.mass_kg / units.SOLAR_MASS_KG:.3f} M☉) excede o limite superior para anãs marrons (~0.08 M☉)."
            )
        if star.effective_temperature_k is not None and star.effective_temperature_k > 3200.0:
            warnings.append(
                f"Temperatura efetiva ({star.effective_temperature_k:.0f} K) muito alta para anã marrom (< 3000 K)."
            )

    elif star.kind == "Star":
        if star.effective_temperature_k is not None:
            if star.effective_temperature_k < 1800.0:
                warnings.append(
                    f"Temperatura efetiva ({star.effective_temperature_k:.0f} K) incomumente baixa para estrela convencional."
                )
            elif star.effective_temperature_k > 65000.0:
                warnings.append(
                    f"Temperatura efetiva ({star.effective_temperature_k:.0f} K) extraordinariamente alta para estrelas conhecidas."
                )

    return warnings


def curate_planet(planet: Planet) -> List[str]:
    warnings: List[str] = []

    if planet.mass_kg > 0.0 and planet.equatorial_radius_m is not None and planet.equatorial_radius_m > 0.0:
        r_eq = planet.equatorial_radius_m
        r_pol = planet.polar_radius_m if (planet.polar_radius_m is not None and planet.polar_radius_m > 0.0) else r_eq
        vol = (4.0 / 3.0) * math.pi * (r_eq ** 2) * r_pol
        dens = planet.mass_kg / vol
        stats = reference_data.get_planet_statistics(planet.kind)
        dens_stat = stats.get("density_kg_per_m3") or stats.get("density")
        if dens_stat:
            min_d = dens_stat.get("min")
            max_d = dens_stat.get("max")
            if min_d is not None and dens < min_d * 0.25:
                warnings.append(
                    f"Densidade média ({dens:.1f} kg/m³) significativamente abaixo da faixa esperada para '{planet.kind}' (mínimo típico ~{min_d:.1f} kg/m³)."
                )
            if max_d is not None and dens > max_d * 4.0:
                warnings.append(
                    f"Densidade média ({dens:.1f} kg/m³) significativamente acima da faixa esperada para '{planet.kind}' (máximo típico ~{max_d:.1f} kg/m³)."
                )

    if planet.equatorial_radius_m is not None and planet.polar_radius_m is not None:
        if planet.polar_radius_m > planet.equatorial_radius_m:
            warnings.append(
                f"Raio polar ({planet.polar_radius_m:.0f} m) maior que o raio equatorial ({planet.equatorial_radius_m:.0f} m), formato prolato anômalo para corpos em rotação."
            )
        elif planet.equatorial_radius_m > 0.0:
            f = (planet.equatorial_radius_m - planet.polar_radius_m) / planet.equatorial_radius_m
            if f > 0.5:
                warnings.append(f"Achatamento polar excessivo (f = {f:.3f} > 0.50).")
            if (
                planet.rotation_period_s is not None
                and planet.rotation_period_s > 0.0
                and planet.mass_kg > 0.0
            ):
                omega = (2.0 * math.pi) / planet.rotation_period_s
                mu = physics_lite.gravitational_parameter(planet.mass_kg)
                q = (omega * omega * (planet.equatorial_radius_m ** 3)) / mu
                limit_f = 0.5 * q + 1.5 * (planet.oblateness_j2 or 0.0) + 0.12
                if q > 0.0 and f > limit_f:
                    warnings.append(
                        f"Achatamento polar ({f:.3f}) significativamente superior ao valor sustentável pela rotação."
                    )

    if planet.kind == "IcyBody":
        if planet.geometric_albedo is not None and planet.geometric_albedo < 0.20:
            warnings.append(
                f"Albedo geométrico ({planet.geometric_albedo:.2f}) incomumente baixo para corpo gelado (IcyBody, esperado >= 0.20)."
            )
        if planet.bond_albedo is not None and planet.bond_albedo < 0.20:
            warnings.append(
                f"Albedo de Bond ({planet.bond_albedo:.2f}) incomumente baixo para corpo gelado (IcyBody, esperado >= 0.20)."
            )
    elif planet.kind in ("GasGiant", "IceGiant"):
        if planet.geometric_albedo is not None and planet.geometric_albedo < 0.15:
            warnings.append(
                f"Albedo geométrico ({planet.geometric_albedo:.2f}) incomumente baixo para gigante gasoso/gelado."
            )
        if planet.bond_albedo is not None and planet.bond_albedo < 0.15:
            warnings.append(
                f"Albedo de Bond ({planet.bond_albedo:.2f}) incomumente baixo para gigante gasoso/gelado."
            )

    if planet.geometric_albedo is not None and planet.bond_albedo is not None:
        if planet.bond_albedo > 1.8 * planet.geometric_albedo + 0.15:
            warnings.append(
                f"Albedo de Bond ({planet.bond_albedo:.2f}) desproporcionalmente maior que o albedo geométrico ({planet.geometric_albedo:.2f})."
            )

    if planet.axial_tilt_rad is not None:
        tilt_deg = math.degrees(planet.axial_tilt_rad) % 360.0
        if 90.0 < tilt_deg < 270.0:
            warnings.append(
                f"Obliquidade axial de {tilt_deg:.1f}° indica rotação retrógrada (incomum, mas fisicamente plausível)."
            )

    return warnings


def curate_atmosphere(
    atmosphere: Atmosphere,
    components: Optional[List[AtmosphereGasComponent]] = None,
    planet_kind: Optional[str] = None,
) -> List[str]:
    warnings: List[str] = []

    resolved_kind = planet_kind
    if resolved_kind is None and atmosphere.planet_id:
        p = cache.get_entity(atmosphere.planet_id)
        if p:
            resolved_kind = p.get("kind")

    if resolved_kind == "DwarfPlanet" and atmosphere.pressure_pa > 1.0e4:
        warnings.append(
            f"Pressão superficial ({atmosphere.pressure_pa:.0f} Pa) excepcionalmente elevada para planeta anão."
        )
    elif resolved_kind == "Telluric" and atmosphere.pressure_pa > 2.5e7:
        warnings.append(
            f"Pressão superficial ({units.pa_to_atm(atmosphere.pressure_pa):.1f} atm) extremamente alta para corpo telúrico."
        )

    if atmosphere.greenhouse_effect_k > 450.0:
        warnings.append(
            f"Efeito estufa térmico ({atmosphere.greenhouse_effect_k:.1f} K) é extraordinariamente severo."
        )

    if atmosphere.lapse_rate_k_per_m > 0.035:
        warnings.append(
            f"Gradiente térmico vertical (lapse rate = {atmosphere.lapse_rate_k_per_m * 1000.0:.2f} K/km) excessivamente íngreme para convecção usual."
        )
    elif atmosphere.lapse_rate_k_per_m < 0.0:
        warnings.append(
            f"Gradiente térmico vertical negativo (lapse rate = {atmosphere.lapse_rate_k_per_m * 1000.0:.2f} K/km) indica inversão térmica média global permanente."
        )

    if components:
        total_pct = sum(c.percentage for c in components if math.isfinite(c.percentage))
        if total_pct < 95.0:
            warnings.append(
                f"Soma das frações gasosas ({total_pct:.2f}%) não cobre a quase totalidade da composição (< 95%)."
            )

        mean_molar = 0.0
        if total_pct > 0.0:
            for c in components:
                if math.isfinite(c.percentage):
                    f_clean = c.formula.strip().upper()
                    m = ROUGH_MOLAR_MASS.get(f_clean, 28.0) * 0.001
                    mean_molar += m * (c.percentage / total_pct)

            if mean_molar > 0.0 and atmosphere.pressure_pa > 0.0:
                rho = (atmosphere.pressure_pa * mean_molar) / (physics_lite.UNIVERSAL_GAS_CONSTANT * 288.15)
                if rho > 1200.0:
                    warnings.append(
                        f"Densidade estimada da base atmosférica ({rho:.1f} kg/m³) aproxima-se da densidade de líquidos compactos."
                    )

    return warnings


def curate_barycenter(barycenter: Barycenter) -> List[str]:
    warnings: List[str] = []

    pri_id = (
        barycenter.primary_star_id
        or barycenter.primary_planet_id
        or barycenter.primary_barycenter_id
    )
    sec_id = (
        barycenter.secondary_star_id
        or barycenter.secondary_planet_id
        or barycenter.secondary_barycenter_id
    )

    m_pri: Optional[float] = None
    m_sec: Optional[float] = None

    if pri_id:
        e1 = cache.get_entity(pri_id)
        if e1 and e1.get("mass_kg") is not None:
            m_pri = e1["mass_kg"]
    if sec_id:
        e2 = cache.get_entity(sec_id)
        if e2 and e2.get("mass_kg") is not None:
            m_sec = e2["mass_kg"]

    if m_pri is not None and m_sec is not None:
        if m_sec > m_pri:
            warnings.append(
                f"Membro secundário tem massa ({m_sec:.2e} kg) superior à do primário ({m_pri:.2e} kg)."
            )

    if barycenter.internal_eccentricity > 0.92:
        warnings.append(
            f"Excentricidade interna muito alta (e = {barycenter.internal_eccentricity:.3f}), risco elevado de instabilidade orbital."
        )

    return warnings


def curate_star_system(_system: StarSystem) -> List[str]:
    return []
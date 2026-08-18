from dataclasses import dataclass
import math
from typing import Any, Dict, List, Optional, Tuple
import physics_lite
import reference_data


@dataclass
class SuggestionResult:
    suggested_fields: Dict[str, Any]
    reference_names: List[str]
    note: str
    inferred_kind: Optional[str] = None
    candidate_index: int = 0
    total_candidates: int = 0


LOG_SCALE_FIELDS = {
    "mass_kg",
    "radius_m",
    "equatorial_radius_m",
    "polar_radius_m",
    "effective_temperature_k",
    "rotation_period_s",
    "semi_major_axis_m",
    "pressure_pa",
}

LINEAR_WEIGHTS = {
    "geometric_albedo": 1.0,
    "bond_albedo": 1.0,
    "thermal_inertia": 1.0,
    "axial_tilt_rad": 0.5,
    "solstice_true_anomaly_rad": 0.3,
    "oblateness_j2": 10.0,
    "eccentricity": 2.0,
    "inclination_rad": 0.5,
}


def _calc_field_distance(key: str, v1: float, v2: float) -> float:
    if v1 is None or v2 is None:
        return 0.0
    if not math.isfinite(v1) or not math.isfinite(v2):
        return 0.0
    if key in LOG_SCALE_FIELDS:
        if v1 <= 0.0 or v2 <= 0.0:
            return 10.0
        return abs(math.log10(v1) - math.log10(v2))
    weight = LINEAR_WEIGHTS.get(key, 1.0)
    return abs(v1 - v2) * weight


def _match_score(known: Dict[str, Any], item: Dict[str, Any]) -> Tuple[float, int]:
    total_dist = 0.0
    matched_count = 0
    for k, val in known.items():
        if val is None or k in ("kind", "name", "id"):
            continue
        v_item = item.get(k)
        if v_item is None and k == "equatorial_radius_m":
            v_item = item.get("radius_m")
        if v_item is not None:
            try:
                total_dist += _calc_field_distance(k, float(val), float(v_item))
                matched_count += 1
            except (ValueError, TypeError):
                total_dist += 2.0
        else:
            total_dist += 2.0
    return total_dist, matched_count


def _find_top_candidates(
    dataset: List[Dict[str, Any]],
    known: Dict[str, Any],
    kind_filter: Optional[str] = None,
    k: int = 5,
) -> Tuple[List[Dict[str, Any]], Optional[str]]:
    items = dataset
    if kind_filter:
        filtered = [x for x in items if str(x.get("kind", "")).strip().lower() == kind_filter.strip().lower()]
        if filtered:
            items = filtered

    scored = []
    has_known_numeric = any(
        v is not None for k, v in known.items() if k not in ("kind", "name", "id")
    )

    for item in items:
        if has_known_numeric:
            score, matched = _match_score(known, item)
            penalty = 1.0 / (matched + 1.0)
            final_score = score + penalty
        else:
            final_score = 0.0
        scored.append((final_score, item))

    scored.sort(key=lambda pair: pair[0])
    top = [pair[1] for pair in scored[:k]]

    inferred_kind = None
    if top:
        inferred_kind = top[0].get("kind")

    return top, inferred_kind


def suggest_orbital_context(
    body_kind: Optional[str],
    parent_data: Dict[str, Any],
    known_orbit: Optional[Dict[str, Any]] = None,
    cursor: int = 0,
) -> Tuple[Dict[str, Any], str]:
    suggested: Dict[str, Any] = {}
    known = known_orbit or {}

    p_type = parent_data.get("entity_type", "")
    p_radius = parent_data.get("radius_m") or parent_data.get("equatorial_radius_m")
    p_temp = parent_data.get("effective_temperature_k")
    p_mass = parent_data.get("mass_kg")
    p_name = parent_data.get("name", "Primário")

    is_star_parent = (
        (p_type == "Star")
        or (p_temp is not None and p_temp > 0.0)
        or (p_radius is not None and p_radius > 1.0e8)
    )

    note_context = ""

    if is_star_parent:
        if p_radius and p_temp:
            lum = physics_lite.stellar_luminosity(p_radius, p_temp)
        elif p_mass and p_mass > 0.0:
            lum = physics_lite.SOLAR_LUMINOSITY * ((p_mass / physics_lite.SOLAR_MASS) ** 3.5)
        else:
            lum = physics_lite.SOLAR_LUMINOSITY

        hz_in, hz_out = physics_lite.habitable_zone_boundaries(lum)
        if hz_in <= 0.0:
            hz_in = 0.95 * physics_lite.ASTRONOMICAL_UNIT
            hz_out = 1.37 * physics_lite.ASTRONOMICAL_UNIT

        frost_line = 2.7 * physics_lite.ASTRONOMICAL_UNIT * math.sqrt(
            max(1e-4, lum / physics_lite.SOLAR_LUMINOSITY)
        )

        k = (body_kind or "").strip()
        if k in ("Telluric", "CarbonPlanet"):
            sma_candidates = [
                0.5 * (hz_in + hz_out),
                0.85 * hz_in,
                1.15 * hz_out,
                0.70 * hz_in,
                1.25 * hz_out,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita sugerida em relação à Zona Habitável ({hz_in / physics_lite.ASTRONOMICAL_UNIT:.2f} - {hz_out / physics_lite.ASTRONOMICAL_UNIT:.2f} AU) de {p_name}."
        elif k in ("GasGiant", "IceGiant"):
            sma_candidates = [
                1.30 * frost_line,
                1.90 * frost_line,
                2.70 * frost_line,
                4.00 * frost_line,
                0.05 * physics_lite.ASTRONOMICAL_UNIT,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita sugerida em relação à Linha de Gelo (~{frost_line / physics_lite.ASTRONOMICAL_UNIT:.2f} AU) de {p_name}."
        elif k in ("DwarfPlanet", "IcyBody"):
            sma_candidates = [
                2.50 * frost_line,
                3.80 * frost_line,
                5.50 * frost_line,
                8.20 * frost_line,
                12.0 * frost_line,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita externa sugerida além da Linha de Gelo (~{frost_line / physics_lite.ASTRONOMICAL_UNIT:.2f} AU)."
        elif k == "Chthonian":
            sma_candidates = [
                0.025 * physics_lite.ASTRONOMICAL_UNIT,
                0.045 * physics_lite.ASTRONOMICAL_UNIT,
                0.080 * physics_lite.ASTRONOMICAL_UNIT,
                0.120 * hz_in,
                0.035 * physics_lite.ASTRONOMICAL_UNIT,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita ultra-curta sugerida para núcleo ctoniano de {p_name}."
        else:
            sma_candidates = [
                0.5 * (hz_in + hz_out),
                1.3 * frost_line,
                0.8 * hz_in,
                2.2 * frost_line,
                0.15 * hz_in,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita configurada em relação à radiação estelar de {p_name}."
    else:
        ref_rad = p_radius if (p_radius and p_radius > 0.0) else physics_lite.EARTH_RADIUS
        sma_candidates = [
            9.5 * ref_rad,
            18.0 * ref_rad,
            32.0 * ref_rad,
            55.0 * ref_rad,
            80.0 * ref_rad,
        ]
        chosen_sma = sma_candidates[cursor % len(sma_candidates)]
        note_context = f"Órbita circumplanetária sugerida em torno de {p_name}."

    if known.get("semi_major_axis_m") is None:
        suggested["semi_major_axis_m"] = chosen_sma

    k = (body_kind or "").strip()
    if k == "DwarfPlanet":
        ecc_candidates = [0.12, 0.21, 0.07, 0.28, 0.16]
        inc_candidates = [math.radians(11.0), math.radians(22.0), math.radians(7.0), math.radians(17.0), math.radians(14.0)]
    elif k in ("GasGiant", "IceGiant"):
        ecc_candidates = [0.032, 0.048, 0.012, 0.055, 0.022]
        inc_candidates = [math.radians(1.3), math.radians(2.5), math.radians(0.5), math.radians(1.8), math.radians(3.1)]
    else:
        ecc_candidates = [0.016, 0.028, 0.007, 0.045, 0.021]
        inc_candidates = [math.radians(1.8), math.radians(3.4), math.radians(0.8), math.radians(5.1), math.radians(2.2)]

    if known.get("eccentricity") is None:
        suggested["eccentricity"] = ecc_candidates[cursor % len(ecc_candidates)]
    if known.get("inclination_rad") is None:
        suggested["inclination_rad"] = inc_candidates[cursor % len(inc_candidates)]
    if known.get("longitude_ascending_node_rad") is None:
        lan_candidates = [math.radians(0.0), math.radians(45.0), math.radians(110.0), math.radians(225.0), math.radians(310.0)]
        suggested["longitude_ascending_node_rad"] = lan_candidates[cursor % len(lan_candidates)]
    if known.get("argument_periapsis_rad") is None:
        arg_candidates = [math.radians(0.0), math.radians(65.0), math.radians(140.0), math.radians(280.0), math.radians(195.0)]
        suggested["argument_periapsis_rad"] = arg_candidates[cursor % len(arg_candidates)]
    if known.get("mean_anomaly_at_epoch_rad") is None:
        m0_candidates = [math.radians(0.0), math.radians(90.0), math.radians(180.0), math.radians(270.0), math.radians(45.0)]
        suggested["mean_anomaly_at_epoch_rad"] = m0_candidates[cursor % len(m0_candidates)]

    return suggested, note_context


def suggest_star_fill(
    known_fields: Dict[str, Any],
    cursor: int = 0,
    parent_data: Optional[Dict[str, Any]] = None,
) -> SuggestionResult:
    dataset = reference_data.get_all_stars()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(dataset, known_fields, kind_filter, k=5)

    ref_kind = kind_filter or inferred_kind or "Star"
    ref_name = "Corpo Estelar Procedural"

    if top_candidates:
        idx = cursor % len(top_candidates)
        chosen = top_candidates[idx]
        ref_name = chosen.get("name") or chosen.get("nome") or ref_name
        ref_kind = chosen.get("kind", ref_kind)
    else:
        chosen = {}
        idx = cursor

    suggested: Dict[str, Any] = {}

    mass_val = known_fields.get("mass_kg")
    rad_val = known_fields.get("radius_m")
    temp_val = known_fields.get("effective_temperature_k")
    rot_val = known_fields.get("rotation_period_s")
    tilt_val = known_fields.get("axial_tilt_rad")
    j2_val = known_fields.get("oblateness_j2")

    if ref_kind == "Star":
        if mass_val is None:
            mass_val = chosen.get("mass_kg") or chosen.get("massa_kg")
            if mass_val is None:
                mass_options = [1.9885e30, 0.85 * 1.9885e30, 1.4 * 1.9885e30, 0.3 * 1.9885e30, 2.2 * 1.9885e30]
                mass_val = mass_options[cursor % len(mass_options)]
            suggested["mass_kg"] = mass_val

        if rad_val is None:
            rad_val = chosen.get("radius_m") or chosen.get("raio_m")
            if rad_val is None and mass_val:
                rad_val = physics_lite.stellar_main_sequence_radius(mass_val)
            if rad_val is None:
                rad_val = physics_lite.SOLAR_RADIUS
            suggested["radius_m"] = rad_val

        if temp_val is None:
            temp_val = chosen.get("effective_temperature_k") or chosen.get("temperatura_efetiva_k")
            if temp_val is None and mass_val and rad_val:
                temp_val = physics_lite.stellar_main_sequence_temperature(mass_val, rad_val)
            if temp_val is None:
                temp_val = physics_lite.SOLAR_TEMPERATURE
            suggested["effective_temperature_k"] = temp_val

        if rot_val is None:
            rot_val = chosen.get("rotation_period_s") or chosen.get("periodo_rotacao_s")
            if rot_val is None and mass_val:
                m_rel = mass_val / physics_lite.SOLAR_MASS
                rot_val = 25.0 * 86400.0 * (m_rel ** 0.4)
            if rot_val is None:
                rot_val = 25.0 * 86400.0
            suggested["rotation_period_s"] = rot_val

        if tilt_val is None:
            tilt_val = chosen.get("axial_tilt_rad") or chosen.get("obliquidade_rad")
            if tilt_val is None:
                tilt_val = math.radians(7.25)
            suggested["axial_tilt_rad"] = tilt_val

        if j2_val is None:
            j2_val = chosen.get("oblateness_j2") or chosen.get("achatamento_j2")
            if j2_val is None and mass_val and rad_val and rot_val:
                j2_val = physics_lite.oblateness_j2_from_rotation(mass_val, rad_val, rot_val, love_k2=0.05)
            if j2_val is None or j2_val <= 0.0:
                j2_val = 2.0e-7
            suggested["oblateness_j2"] = j2_val

    elif ref_kind == "WhiteDwarf":
        if mass_val is None:
            mass_val = chosen.get("mass_kg") or chosen.get("massa_kg") or (0.6 * physics_lite.SOLAR_MASS)
            suggested["mass_kg"] = mass_val
        if rad_val is None:
            rad_val = chosen.get("radius_m") or chosen.get("raio_m") or (0.009 * physics_lite.SOLAR_RADIUS)
            suggested["radius_m"] = rad_val
        if temp_val is None:
            temp_options = [25000.0, 15000.0, 10000.0, 32000.0, 8000.0]
            temp_val = chosen.get("effective_temperature_k") or chosen.get("temperatura_efetiva_k") or temp_options[cursor % len(temp_options)]
            suggested["effective_temperature_k"] = temp_val
        if rot_val is None:
            rot_val = chosen.get("rotation_period_s") or chosen.get("periodo_rotacao_s") or 86400.0
            suggested["rotation_period_s"] = rot_val
        if tilt_val is None:
            suggested["axial_tilt_rad"] = 0.0
        if j2_val is None:
            suggested["oblateness_j2"] = 1.0e-6

    elif ref_kind == "NeutronStar":
        if mass_val is None:
            mass_val = chosen.get("mass_kg") or chosen.get("massa_kg") or (1.4 * physics_lite.SOLAR_MASS)
            suggested["mass_kg"] = mass_val
        if rad_val is None:
            rad_val = chosen.get("radius_m") or chosen.get("raio_m") or 12000.0
            suggested["radius_m"] = rad_val
        if temp_val is None:
            temp_options = [100000.0, 300000.0, 50000.0, 600000.0, 150000.0]
            temp_val = chosen.get("effective_temperature_k") or chosen.get("temperatura_efetiva_k") or temp_options[cursor % len(temp_options)]
            suggested["effective_temperature_k"] = temp_val
        if rot_val is None:
            rot_options = [0.033, 0.003, 0.1, 0.015, 0.5]
            rot_val = chosen.get("rotation_period_s") or chosen.get("periodo_rotacao_s") or rot_options[cursor % len(rot_options)]
            suggested["rotation_period_s"] = rot_val
        if tilt_val is None:
            suggested["axial_tilt_rad"] = 0.0
        if j2_val is None:
            suggested["oblateness_j2"] = 1.0e-4

    elif ref_kind == "BlackHole":
        if mass_val is None:
            mass_val = chosen.get("mass_kg") or chosen.get("massa_kg") or (10.0 * physics_lite.SOLAR_MASS)
            suggested["mass_kg"] = mass_val
        if rad_val is None and mass_val:
            rad_val = physics_lite.schwarzschild_radius(mass_val)
            suggested["radius_m"] = rad_val

    elif ref_kind == "BrownDwarf":
        if mass_val is None:
            mass_val = chosen.get("mass_kg") or chosen.get("massa_kg") or (0.04 * physics_lite.SOLAR_MASS)
            suggested["mass_kg"] = mass_val
        if rad_val is None:
            rad_val = chosen.get("radius_m") or chosen.get("raio_m") or physics_lite.JUPITER_RADIUS
            suggested["radius_m"] = rad_val
        if temp_val is None:
            temp_options = [940.0, 1400.0, 1800.0, 750.0, 2200.0]
            temp_val = chosen.get("effective_temperature_k") or chosen.get("temperatura_efetiva_k") or temp_options[cursor % len(temp_options)]
            suggested["effective_temperature_k"] = temp_val
        if rot_val is None:
            rot_val = chosen.get("rotation_period_s") or chosen.get("periodo_rotacao_s") or 28800.0
            suggested["rotation_period_s"] = rot_val
        if tilt_val is None:
            suggested["axial_tilt_rad"] = math.radians(3.0)
        if j2_val is None:
            suggested["oblateness_j2"] = 0.015

    else:
        for field_name in (
            "mass_kg",
            "radius_m",
            "effective_temperature_k",
            "rotation_period_s",
            "axial_tilt_rad",
            "oblateness_j2",
        ):
            if known_fields.get(field_name) is None:
                val = chosen.get(field_name)
                if val is not None:
                    suggested[field_name] = val

    final_suggested = {k: v for k, v in suggested.items() if known_fields.get(k) is None and v is not None}

    note = f"Baseado em {ref_name} ({ref_kind}). Variação procedural {idx + 1} de {max(1, len(top_candidates))}."

    if parent_data:
        orbit_suggested, orbit_note = suggest_orbital_context(
            body_kind=ref_kind,
            parent_data=parent_data,
            known_orbit=known_fields,
            cursor=cursor,
        )
        for ok, ov in orbit_suggested.items():
            if known_fields.get(ok) is None:
                final_suggested[ok] = ov
        if orbit_note:
            note += f" {orbit_note}"

    return SuggestionResult(
        suggested_fields=final_suggested,
        reference_names=[ref_name],
        note=note,
        inferred_kind=ref_kind,
        candidate_index=idx,
        total_candidates=max(1, len(top_candidates)),
    )


def suggest_planet_fill(
    known_fields: Dict[str, Any],
    cursor: int = 0,
    parent_data: Optional[Dict[str, Any]] = None,
) -> SuggestionResult:
    dataset = reference_data.get_all_planets()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(dataset, known_fields, kind_filter, k=5)

    ref_kind = kind_filter or inferred_kind or "Telluric"
    ref_name = "Corpo Planetário Procedural"

    if top_candidates:
        idx = cursor % len(top_candidates)
        chosen = top_candidates[idx]
        ref_name = chosen.get("name") or chosen.get("nome") or ref_name
        ref_kind = chosen.get("kind", ref_kind)
    else:
        chosen = {}
        idx = cursor

    suggested: Dict[str, Any] = {}

    mass_val = known_fields.get("mass_kg")
    r_eq_val = known_fields.get("equatorial_radius_m") or known_fields.get("radius_m")
    r_pol_val = known_fields.get("polar_radius_m")
    rot_val = known_fields.get("rotation_period_s")
    tilt_val = known_fields.get("axial_tilt_rad")
    geo_val = known_fields.get("geometric_albedo")
    bond_val = known_fields.get("bond_albedo")
    ti_val = known_fields.get("thermal_inertia")
    solst_val = known_fields.get("solstice_true_anomaly_rad")
    j2_val = known_fields.get("oblateness_j2")

    if mass_val is None:
        mass_val = chosen.get("mass_kg") or chosen.get("massa_kg")
        if mass_val is None:
            if ref_kind == "Telluric":
                mass_options = [1.0 * physics_lite.EARTH_MASS, 0.65 * physics_lite.EARTH_MASS, 2.5 * physics_lite.EARTH_MASS, 0.107 * physics_lite.EARTH_MASS, 0.012 * physics_lite.EARTH_MASS]
                mass_val = mass_options[cursor % len(mass_options)]
            elif ref_kind == "GasGiant":
                mass_options = [1.0 * physics_lite.JUPITER_MASS, 0.3 * physics_lite.JUPITER_MASS, 1.8 * physics_lite.JUPITER_MASS, 2.5 * physics_lite.JUPITER_MASS, 0.6 * physics_lite.JUPITER_MASS]
                mass_val = mass_options[cursor % len(mass_options)]
            elif ref_kind == "IceGiant":
                mass_options = [1.0 * physics_lite.NEPTUNE_MASS, 0.85 * physics_lite.NEPTUNE_MASS, 1.5 * physics_lite.NEPTUNE_MASS, 2.2 * physics_lite.NEPTUNE_MASS, 0.5 * physics_lite.NEPTUNE_MASS]
                mass_val = mass_options[cursor % len(mass_options)]
            elif ref_kind in ("DwarfPlanet", "IcyBody"):
                mass_options = [1.3e22, 1.6e22, 9.4e20, 4.0e21, 4.8e22]
                mass_val = mass_options[cursor % len(mass_options)]
            elif ref_kind == "CarbonPlanet":
                mass_options = [1.0 * physics_lite.EARTH_MASS, 2.0 * physics_lite.EARTH_MASS, 7.7 * physics_lite.EARTH_MASS]
                mass_val = mass_options[cursor % len(mass_options)]
            elif ref_kind == "Chthonian":
                mass_options = [1.0 * physics_lite.EARTH_MASS, 5.0 * physics_lite.EARTH_MASS, 10.0 * physics_lite.EARTH_MASS]
                mass_val = mass_options[cursor % len(mass_options)]
            else:
                mass_val = physics_lite.EARTH_MASS
        suggested["mass_kg"] = mass_val

    if r_eq_val is None:
        r_eq_val = chosen.get("equatorial_radius_m") or chosen.get("raio_equatorial_m") or chosen.get("radius_m") or chosen.get("raio_m")
        if r_eq_val is None:
            if r_pol_val is not None:
                r_eq_val = r_pol_val * 1.0034
            elif mass_val is not None:
                if ref_kind == "Telluric":
                    r_eq_val = physics_lite.telluric_radius_from_mass(mass_val)
                elif ref_kind == "GasGiant":
                    r_eq_val = physics_lite.gas_giant_radius_from_mass(mass_val)
                elif ref_kind == "IceGiant":
                    r_eq_val = physics_lite.ice_giant_radius_from_mass(mass_val)
                elif ref_kind in ("DwarfPlanet", "IcyBody"):
                    vol = mass_val / 2000.0
                    r_eq_val = (vol / ((4.0 / 3.0) * math.pi)) ** (1.0 / 3.0)
                elif ref_kind == "Chthonian":
                    vol = mass_val / 7500.0
                    r_eq_val = (vol / ((4.0 / 3.0) * math.pi)) ** (1.0 / 3.0)
                elif ref_kind == "CarbonPlanet":
                    vol = mass_val / 4200.0
                    r_eq_val = (vol / ((4.0 / 3.0) * math.pi)) ** (1.0 / 3.0)
                else:
                    r_eq_val = physics_lite.EARTH_EQUATORIAL_RADIUS
            else:
                r_eq_val = physics_lite.EARTH_EQUATORIAL_RADIUS
        suggested["equatorial_radius_m"] = r_eq_val

    if rot_val is None:
        rot_val = chosen.get("rotation_period_s") or chosen.get("periodo_rotacao_s")
        if rot_val is None:
            if ref_kind == "Telluric":
                rot_options = [86164.0, 50000.0, 100000.0, 150000.0, 36000.0]
            elif ref_kind == "GasGiant":
                rot_options = [35730.0, 38362.0, 42000.0, 32000.0, 48000.0]
            elif ref_kind == "IceGiant":
                rot_options = [58000.0, 62000.0, 54000.0, 68000.0, 45000.0]
            elif ref_kind == "DwarfPlanet":
                rot_options = [32667.0, 82175.0, 14095.0, 136390.0, 45000.0]
            elif ref_kind == "IcyBody":
                rot_options = [152853.0, 306822.0, 618153.0, 118386.0, 1377648.0]
            elif ref_kind == "Chthonian":
                rot_options = [86400.0, 43200.0, 60000.0, 120000.0, 30000.0]
            elif ref_kind == "CarbonPlanet":
                rot_options = [60000.0, 72000.0, 48000.0, 90000.0, 36000.0]
            else:
                rot_options = [36000.0, 80000.0, 50000.0, 100000.0, 24000.0]
            rot_val = rot_options[cursor % len(rot_options)]
        suggested["rotation_period_s"] = rot_val

    if j2_val is None:
        j2_val = chosen.get("oblateness_j2") or chosen.get("achatamento_j2")
        if j2_val is None and mass_val and r_eq_val and rot_val:
            k2 = 0.52 if ref_kind in ("GasGiant", "IceGiant") else 0.93
            j2_val = physics_lite.oblateness_j2_from_rotation(mass_val, r_eq_val, rot_val, love_k2=k2)
        if j2_val is None or j2_val <= 0.0:
            j2_val = 0.0147 if ref_kind == "GasGiant" else 0.00108
        suggested["oblateness_j2"] = j2_val

    if r_pol_val is None:
        r_pol_val = chosen.get("polar_radius_m") or chosen.get("raio_polar_m")
        if r_pol_val is None and mass_val and r_eq_val and rot_val:
            f = physics_lite.rotational_flattening(mass_val, r_eq_val, rot_val, j2=j2_val or 0.0)
            r_pol_val = r_eq_val * (1.0 - f)
        elif r_pol_val is None and r_eq_val:
            r_pol_val = r_eq_val * 0.9966
        suggested["polar_radius_m"] = r_pol_val

    if tilt_val is None:
        tilt_val = chosen.get("axial_tilt_rad") or chosen.get("obliquidade_rad")
        if tilt_val is None:
            if ref_kind == "Telluric":
                tilt_options = [math.radians(23.44), math.radians(25.19), math.radians(1.5), math.radians(12.0), math.radians(28.0)]
            elif ref_kind == "GasGiant":
                tilt_options = [math.radians(3.13), math.radians(26.73), math.radians(2.5), math.radians(15.0), math.radians(8.0)]
            elif ref_kind == "IceGiant":
                tilt_options = [math.radians(28.32), math.radians(97.77), math.radians(35.0), math.radians(45.0), math.radians(82.0)]
            elif ref_kind in ("DwarfPlanet", "IcyBody"):
                tilt_options = [math.radians(4.0), math.radians(0.5), math.radians(24.0), math.radians(1.0), math.radians(18.0)]
            else:
                tilt_options = [math.radians(5.0), math.radians(15.0), math.radians(0.0), math.radians(22.0), math.radians(10.0)]
            tilt_val = tilt_options[cursor % len(tilt_options)]
        suggested["axial_tilt_rad"] = tilt_val

    if geo_val is None:
        geo_val = chosen.get("geometric_albedo") or chosen.get("albedo_geometrico")
        if geo_val is None:
            if ref_kind == "Telluric":
                geo_options = [0.367, 0.142, 0.170, 0.250, 0.300]
            elif ref_kind == "GasGiant":
                geo_options = [0.503, 0.499, 0.450, 0.530, 0.480]
            elif ref_kind == "IceGiant":
                geo_options = [0.488, 0.422, 0.460, 0.510, 0.390]
            elif ref_kind == "DwarfPlanet":
                geo_options = [0.520, 0.090, 0.660, 0.770, 0.350]
            elif ref_kind == "IcyBody":
                geo_options = [0.670, 0.430, 0.630, 0.756, 0.550]
            elif ref_kind == "Chthonian":
                geo_options = [0.100, 0.150, 0.080, 0.120, 0.180]
            elif ref_kind == "CarbonPlanet":
                geo_options = [0.200, 0.300, 0.150, 0.220, 0.280]
            else:
                geo_options = [0.400, 0.050, 0.300, 0.500, 0.200]
            geo_val = geo_options[cursor % len(geo_options)]
        suggested["geometric_albedo"] = geo_val

    if bond_val is None:
        bond_val = chosen.get("bond_albedo") or chosen.get("albedo_bond")
        if bond_val is None and geo_val is not None:
            bond_val = max(0.01, min(0.99, geo_val * 0.85))
        elif bond_val is None:
            bond_val = 0.30
        suggested["bond_albedo"] = bond_val

    if ti_val is None:
        ti_val = chosen.get("thermal_inertia") or chosen.get("inercia_termica")
        if ti_val is None:
            if ref_kind in ("GasGiant", "IceGiant"):
                ti_options = [0.90, 0.85, 0.92, 0.88, 0.95]
            elif ref_kind in ("DwarfPlanet", "IcyBody"):
                ti_options = [0.05, 0.02, 0.08, 0.04, 0.10]
            elif ref_kind == "Chthonian":
                ti_options = [0.15, 0.10, 0.20, 0.12, 0.18]
            elif ref_kind == "CarbonPlanet":
                ti_options = [0.20, 0.25, 0.18, 0.22, 0.30]
            else:
                ti_options = [0.30, 0.25, 0.35, 0.20, 0.40]
            ti_val = ti_options[cursor % len(ti_options)]
        suggested["thermal_inertia"] = ti_val

    if solst_val is None:
        solst_val = chosen.get("solstice_true_anomaly_rad") or chosen.get("anomalia_solsticio_rad")
        if solst_val is None:
            solst_options = [math.radians(90.0), math.radians(0.0), math.radians(180.0), math.radians(270.0), math.radians(45.0)]
            solst_val = solst_options[cursor % len(solst_options)]
        suggested["solstice_true_anomaly_rad"] = solst_val

    final_suggested = {k: v for k, v in suggested.items() if known_fields.get(k) is None and v is not None}

    note = f"Baseado em {ref_name} ({ref_kind}). Variação procedural {idx + 1} de {max(1, len(top_candidates))}."

    if parent_data:
        orbit_suggested, orbit_note = suggest_orbital_context(
            body_kind=ref_kind,
            parent_data=parent_data,
            known_orbit=known_fields,
            cursor=cursor,
        )
        for ok, ov in orbit_suggested.items():
            if known_fields.get(ok) is None:
                final_suggested[ok] = ov
        if orbit_note:
            note += f" {orbit_note}"

    return SuggestionResult(
        suggested_fields=final_suggested,
        reference_names=[ref_name],
        note=note,
        inferred_kind=ref_kind,
        candidate_index=idx,
        total_candidates=max(1, len(top_candidates)),
    )


def suggest_atmosphere_fill(
    planet_kind: Optional[str] = None,
    cursor: int = 0,
    planet_data: Optional[Dict[str, Any]] = None,
) -> SuggestionResult:
    candidates = reference_data.get_atmosphere_archetypes(planet_kind)
    if not candidates:
        candidates = reference_data.get_all_atmospheres()

    if not candidates:
        return SuggestionResult(
            suggested_fields={},
            reference_names=[],
            note="Nenhum arquétipo de atmosfera disponível.",
            inferred_kind=planet_kind,
            candidate_index=0,
            total_candidates=0,
        )

    idx = cursor % len(candidates)
    chosen = candidates[idx]
    arch_name = chosen.get("name") or chosen.get("nome") or "Atmosfera Padrão"

    pres = chosen.get("pressure_pa", 101325.0)
    gh = chosen.get("greenhouse_effect_k", 33.0)
    gamma = chosen.get("lapse_rate_k_per_m", 0.0065)
    comp = chosen.get("composition", [])

    if planet_data:
        m = planet_data.get("mass_kg")
        r = planet_data.get("equatorial_radius_m") or planet_data.get("radius_m")
        if m and r and m > 0.0 and r > 0.0:
            g = physics_lite.surface_gravity(m, r)
            if comp and len(comp) > 0:
                f0 = str(comp[0].get("formula", "")).upper()
                if "H2" in f0:
                    cp = 14300.0
                elif "CO2" in f0:
                    cp = 850.0
                elif "CH4" in f0:
                    cp = 2200.0
                else:
                    cp = 1005.0
                gamma = physics_lite.adiabatic_lapse_rate(g, cp=cp)

    suggested: Dict[str, Any] = {
        "pressure_pa": pres,
        "greenhouse_effect_k": gh,
        "lapse_rate_k_per_m": gamma,
        "composition": comp,
    }

    note = f"Arquétipo: {arch_name}. Opção {idx + 1} de {len(candidates)}."
    if planet_data and planet_data.get("name"):
        note += f" Adaptado para o planeta '{planet_data['name']}' ({planet_kind or 'Desconhecido'})."

    return SuggestionResult(
        suggested_fields=suggested,
        reference_names=[arch_name],
        note=note,
        inferred_kind=planet_kind,
        candidate_index=idx,
        total_candidates=len(candidates),
    )
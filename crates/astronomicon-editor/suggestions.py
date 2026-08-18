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
        if k in item and item[k] is not None:
            total_dist += _calc_field_distance(k, float(val), float(item[k]))
            matched_count += 1
        elif k == "equatorial_radius_m" and "radius_m" in item and item["radius_m"] is not None:
            total_dist += _calc_field_distance(k,
                                               float(val), float(item["radius_m"]))
            matched_count += 1
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
        filtered = [x for x in items if str(
            x.get("kind", "")).lower() == kind_filter.lower()]
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
    p_radius = parent_data.get(
        "radius_m") or parent_data.get("equatorial_radius_m")
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
            lum = physics_lite.SOLAR_LUMINOSITY * \
                ((p_mass / physics_lite.SOLAR_MASS) ** 3.5)
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
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita sugerida dentro da Zona Habitável ({hz_in / physics_lite.ASTRONOMICAL_UNIT:.2f} - {hz_out / physics_lite.ASTRONOMICAL_UNIT:.2f} AU) de {p_name}."
        elif k in ("GasGiant", "IceGiant"):
            sma_candidates = [
                1.25 * frost_line,
                1.80 * frost_line,
                2.60 * frost_line,
                3.80 * frost_line,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita sugerida além da Linha de Gelo (~{frost_line / physics_lite.ASTRONOMICAL_UNIT:.2f} AU) de {p_name}."
        elif k in ("DwarfPlanet", "IcyBody"):
            sma_candidates = [
                2.20 * frost_line,
                3.50 * frost_line,
                5.20 * frost_line,
                8.00 * frost_line,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita externa sugerida para corpo gelado/anão além da Linha de Gelo (~{frost_line / physics_lite.ASTRONOMICAL_UNIT:.2f} AU)."
        elif k == "Chthonian":
            sma_candidates = [
                0.03 * physics_lite.ASTRONOMICAL_UNIT,
                0.06 * physics_lite.ASTRONOMICAL_UNIT,
                0.12 * hz_in,
                0.20 * hz_in,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita extremamente próxima sugerida para remanescente ctoniano de {p_name}."
        else:
            sma_candidates = [
                0.5 * (hz_in + hz_out),
                1.3 * frost_line,
                0.8 * hz_in,
                2.0 * frost_line,
            ]
            chosen_sma = sma_candidates[cursor % len(sma_candidates)]
            note_context = f"Órbita configurada em relação à radiação estelar de {p_name}."
    else:
        ref_rad = p_radius if (p_radius and p_radius >
                               0.0) else physics_lite.EARTH_RADIUS
        sma_candidates = [
            12.0 * ref_rad,
            24.0 * ref_rad,
            45.0 * ref_rad,
            60.0 * ref_rad,
        ]
        chosen_sma = sma_candidates[cursor % len(sma_candidates)]
        note_context = f"Órbita circumplanetária sugerida em torno de {p_name}."

    if known.get("semi_major_axis_m") is None:
        suggested["semi_major_axis_m"] = chosen_sma

    k = (body_kind or "").strip()
    if k == "DwarfPlanet":
        ecc_candidates = [0.15, 0.22, 0.08, 0.28]
        inc_candidates = [math.radians(14.0), math.radians(
            24.0), math.radians(9.0), math.radians(18.0)]
    elif k in ("GasGiant", "IceGiant"):
        ecc_candidates = [0.035, 0.048, 0.012, 0.055]
        inc_candidates = [math.radians(1.2), math.radians(
            2.5), math.radians(0.4), math.radians(1.8)]
    else:
        ecc_candidates = [0.016, 0.028, 0.006, 0.042]
        inc_candidates = [math.radians(1.8), math.radians(
            3.5), math.radians(0.6), math.radians(5.0)]

    if known.get("eccentricity") is None:
        suggested["eccentricity"] = ecc_candidates[cursor %
                                                   len(ecc_candidates)]
    if known.get("inclination_rad") is None:
        suggested["inclination_rad"] = inc_candidates[cursor %
                                                      len(inc_candidates)]
    if known.get("longitude_ascending_node_rad") is None:
        lan_candidates = [math.radians(0.0), math.radians(
            45.0), math.radians(110.0), math.radians(225.0)]
        suggested["longitude_ascending_node_rad"] = lan_candidates[cursor %
                                                                   len(lan_candidates)]
    if known.get("argument_periapsis_rad") is None:
        arg_candidates = [math.radians(0.0), math.radians(
            65.0), math.radians(140.0), math.radians(280.0)]
        suggested["argument_periapsis_rad"] = arg_candidates[cursor %
                                                             len(arg_candidates)]
    if known.get("mean_anomaly_at_epoch_rad") is None:
        m0_candidates = [math.radians(0.0), math.radians(
            90.0), math.radians(180.0), math.radians(270.0)]
        suggested["mean_anomaly_at_epoch_rad"] = m0_candidates[cursor %
                                                               len(m0_candidates)]

    return suggested, note_context


def suggest_star_fill(
    known_fields: Dict[str, Any],
    cursor: int = 0,
    parent_data: Optional[Dict[str, Any]] = None,
) -> SuggestionResult:
    dataset = reference_data.get_all_stars()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(
        dataset, known_fields, kind_filter, k=5)

    if not top_candidates:
        return SuggestionResult(
            suggested_fields={},
            reference_names=[],
            note="Nenhum corpo estelar de referência disponível.",
            inferred_kind=kind_filter,
            candidate_index=0,
            total_candidates=0,
        )

    idx = cursor % len(top_candidates)
    chosen = top_candidates[idx]
    ref_name = chosen.get("name", "Corpo Estelar")
    ref_kind = chosen.get("kind", inferred_kind or "Star")

    suggested: Dict[str, Any] = {}
    for field_name in (
        "kind",
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

    if "kind" in suggested and kind_filter:
        suggested.pop("kind", None)

    note = f"Baseado em {ref_name} ({ref_kind}). Candidato {idx + 1} de {len(top_candidates)}."

    if parent_data:
        orbit_suggested, orbit_note = suggest_orbital_context(
            body_kind=kind_filter or ref_kind,
            parent_data=parent_data,
            known_orbit=known_fields,
            cursor=cursor,
        )
        suggested.update(orbit_suggested)
        if orbit_note:
            note += f" {orbit_note}"

    return SuggestionResult(
        suggested_fields=suggested,
        reference_names=[ref_name],
        note=note,
        inferred_kind=ref_kind,
        candidate_index=idx,
        total_candidates=len(top_candidates),
    )


def suggest_planet_fill(
    known_fields: Dict[str, Any],
    cursor: int = 0,
    parent_data: Optional[Dict[str, Any]] = None,
) -> SuggestionResult:
    dataset = reference_data.get_all_planets()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(
        dataset, known_fields, kind_filter, k=5)

    if not top_candidates:
        return SuggestionResult(
            suggested_fields={},
            reference_names=[],
            note="Nenhum corpo planetário de referência disponível.",
            inferred_kind=kind_filter,
            candidate_index=0,
            total_candidates=0,
        )

    idx = cursor % len(top_candidates)
    chosen = top_candidates[idx]
    ref_name = chosen.get("name", "Corpo Planetário")
    ref_kind = chosen.get("kind", inferred_kind or "Telluric")

    suggested: Dict[str, Any] = {}
    for field_name in (
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
        "oblateness_j2",
    ):
        if known_fields.get(field_name) is None:
            val = chosen.get(field_name)
            if val is None and field_name == "equatorial_radius_m":
                val = chosen.get("radius_m")
            if val is not None:
                suggested[field_name] = val

    if "polar_radius_m" in suggested and suggested["polar_radius_m"] is None:
        eq = known_fields.get("equatorial_radius_m") or suggested.get(
            "equatorial_radius_m")
        if eq is not None:
            suggested["polar_radius_m"] = eq

    if "kind" in suggested and kind_filter:
        suggested.pop("kind", None)

    note = f"Baseado em {ref_name} ({ref_kind}). Candidato {idx + 1} de {len(top_candidates)}."

    if parent_data:
        orbit_suggested, orbit_note = suggest_orbital_context(
            body_kind=kind_filter or ref_kind,
            parent_data=parent_data,
            known_orbit=known_fields,
            cursor=cursor,
        )
        suggested.update(orbit_suggested)
        if orbit_note:
            note += f" {orbit_note}"

    return SuggestionResult(
        suggested_fields=suggested,
        reference_names=[ref_name],
        note=note,
        inferred_kind=ref_kind,
        candidate_index=idx,
        total_candidates=len(top_candidates),
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
    arch_name = chosen.get("nome", "Atmosfera Padrão")

    suggested: Dict[str, Any] = {
        "pressure_pa": chosen.get("pressure_pa"),
        "greenhouse_effect_k": chosen.get("greenhouse_effect_k"),
        "lapse_rate_k_per_m": chosen.get("lapse_rate_k_per_m"),
        "composition": chosen.get("composition", []),
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

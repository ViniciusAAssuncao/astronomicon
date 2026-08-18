from dataclasses import dataclass
import math
from typing import Any, Dict, List, Optional, Tuple
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
            total_dist += _calc_field_distance(k, float(val), float(item["radius_m"]))
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
        filtered = [x for x in items if str(x.get("kind", "")).lower() == kind_filter.lower()]
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


def suggest_star_fill(
    known_fields: Dict[str, Any],
    cursor: int = 0,
) -> SuggestionResult:
    dataset = reference_data.get_all_stars()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(dataset, known_fields, kind_filter, k=5)

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
) -> SuggestionResult:
    dataset = reference_data.get_all_planets()
    kind_filter = known_fields.get("kind")
    top_candidates, inferred_kind = _find_top_candidates(dataset, known_fields, kind_filter, k=5)

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
        eq = known_fields.get("equatorial_radius_m") or suggested.get("equatorial_radius_m")
        if eq is not None:
            suggested["polar_radius_m"] = eq

    if "kind" in suggested and kind_filter:
        suggested.pop("kind", None)

    note = f"Baseado em {ref_name} ({ref_kind}). Candidato {idx + 1} de {len(top_candidates)}."

    return SuggestionResult(
        suggested_fields=suggested,
        reference_names=[ref_name],
        note=note,
        inferred_kind=ref_kind,
        candidate_index=idx,
        total_candidates=len(top_candidates),
    )

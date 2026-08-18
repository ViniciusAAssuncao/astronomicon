import json
import math
import os
import statistics
from typing import Any, Dict, List, Optional, Tuple

_STARS_CACHE: Optional[List[Dict[str, Any]]] = None
_PLANETS_CACHE: Optional[List[Dict[str, Any]]] = None
_ATMOSPHERES_CACHE: Optional[List[Dict[str, Any]]] = None
_BINARIES_CACHE: Optional[List[Dict[str, Any]]] = None


def _resolve_dataset_path(filename: str) -> Optional[str]:
    base_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = [
        os.path.join(base_dir, "data", filename),
        os.path.join(base_dir, filename),
        os.path.join(base_dir, "..", "data", filename),
        os.path.join(base_dir, "..", "..", "data", filename),
    ]
    for candidate in candidates:
        if os.path.isfile(candidate):
            return candidate
    return None


def load_reference_data(
    force_reload: bool = False,
) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]], List[Dict[str, Any]], List[Dict[str, Any]]]:
    global _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE, _BINARIES_CACHE

    if (
        not force_reload
        and _STARS_CACHE is not None
        and _PLANETS_CACHE is not None
        and _ATMOSPHERES_CACHE is not None
        and _BINARIES_CACHE is not None
    ):
        return _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE, _BINARIES_CACHE

    stars: List[Dict[str, Any]] = []
    planets: List[Dict[str, Any]] = []
    atmospheres: List[Dict[str, Any]] = []
    binaries: List[Dict[str, Any]] = []

    stars_path = _resolve_dataset_path("stars.json")
    if stars_path:
        try:
            with open(stars_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    stars = loaded
        except Exception:
            stars = []

    planets_path = _resolve_dataset_path("planets.json")
    if planets_path:
        try:
            with open(planets_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    planets = loaded
        except Exception:
            planets = []

    atmospheres_path = _resolve_dataset_path("atmospheres.json")
    if atmospheres_path:
        try:
            with open(atmospheres_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    atmospheres = loaded
        except Exception:
            atmospheres = []

    binaries_path = _resolve_dataset_path("binaries.json")
    if binaries_path:
        try:
            with open(binaries_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    binaries = loaded
        except Exception:
            binaries = []

    _STARS_CACHE = stars
    _PLANETS_CACHE = planets
    _ATMOSPHERES_CACHE = atmospheres
    _BINARIES_CACHE = binaries
    return _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE, _BINARIES_CACHE


def get_all_stars() -> List[Dict[str, Any]]:
    stars, _, _, _ = load_reference_data()
    return list(stars)


def get_all_planets() -> List[Dict[str, Any]]:
    _, planets, _, _ = load_reference_data()
    return list(planets)


def get_all_atmospheres() -> List[Dict[str, Any]]:
    _, _, atmospheres, _ = load_reference_data()
    return list(atmospheres)


def get_all_binaries() -> List[Dict[str, Any]]:
    _, _, _, binaries = load_reference_data()
    return list(binaries)


def get_atmosphere_archetypes(planet_kind: Optional[str] = None) -> List[Dict[str, Any]]:
    atmospheres = get_all_atmospheres()
    if not planet_kind:
        return atmospheres
    target = planet_kind.strip().lower()
    matched = [
        atm
        for atm in atmospheres
        if any(str(k).strip().lower() == target for k in atm.get("kinds_compativeis", []))
    ]
    return matched if matched else atmospheres


def get_stars_by_kind(kind: str) -> List[Dict[str, Any]]:
    stars = get_all_stars()
    target = kind.strip().lower()
    return [s for s in stars if str(s.get("kind", "")).strip().lower() == target]


def get_planets_by_kind(kind: str) -> List[Dict[str, Any]]:
    planets = get_all_planets()
    target = kind.strip().lower()
    return [p for p in planets if str(p.get("kind", "")).strip().lower() == target]


def get_binaries_by_type(tipo: str) -> List[Dict[str, Any]]:
    binaries = get_all_binaries()
    target = tipo.strip().lower()
    return [b for b in binaries if str(b.get("tipo", "")).strip().lower() == target]


def get_star_kinds() -> List[str]:
    stars = get_all_stars()
    kinds = sorted(list({str(s.get("kind", ""))
                   for s in stars if s.get("kind")}))
    return kinds


def get_planet_kinds() -> List[str]:
    planets = get_all_planets()
    kinds = sorted(list({str(p.get("kind", ""))
                   for p in planets if p.get("kind")}))
    return kinds


def _extract_density(item: Dict[str, Any]) -> Optional[float]:
    if "density_kg_per_m3" in item and item["density_kg_per_m3"] is not None:
        try:
            val = float(item["density_kg_per_m3"])
            if math.isfinite(val) and val > 0.0:
                return val
        except (ValueError, TypeError):
            pass

    if "density" in item and item["density"] is not None:
        try:
            val = float(item["density"])
            if math.isfinite(val) and val > 0.0:
                return val
        except (ValueError, TypeError):
            pass

    mass = item.get("mass_kg")
    radius = item.get("radius_m") or item.get("equatorial_radius_m")
    if mass is not None and radius is not None:
        try:
            m = float(mass)
            r = float(radius)
            if math.isfinite(m) and math.isfinite(r) and m > 0.0 and r > 0.0:
                vol = (4.0 / 3.0) * math.pi * (r ** 3)
                return m / vol
        except (ValueError, TypeError):
            pass

    return None


def _calculate_property_stats(values: List[float]) -> Optional[Dict[str, float]]:
    clean = [v for v in values if v is not None and math.isfinite(v)]
    if not clean:
        return None
    return {
        "min": float(min(clean)),
        "max": float(max(clean)),
        "median": float(statistics.median(clean)),
        "mean": float(statistics.mean(clean)),
        "count": len(clean),
    }


def get_star_statistics(kind: Optional[str] = None) -> Dict[str, Dict[str, float]]:
    items = get_stars_by_kind(kind) if kind is not None else get_all_stars()

    masses: List[float] = []
    radii: List[float] = []
    temps: List[float] = []
    periods: List[float] = []
    densities: List[float] = []
    j2_values: List[float] = []

    for item in items:
        if item.get("mass_kg") is not None:
            try:
                masses.append(float(item["mass_kg"]))
            except (ValueError, TypeError):
                pass

        if item.get("radius_m") is not None:
            try:
                radii.append(float(item["radius_m"]))
            except (ValueError, TypeError):
                pass

        if item.get("effective_temperature_k") is not None:
            try:
                temps.append(float(item["effective_temperature_k"]))
            except (ValueError, TypeError):
                pass

        if item.get("rotation_period_s") is not None:
            try:
                periods.append(float(item["rotation_period_s"]))
            except (ValueError, TypeError):
                pass

        if item.get("oblateness_j2") is not None:
            try:
                j2_values.append(float(item["oblateness_j2"]))
            except (ValueError, TypeError):
                pass

        d = _extract_density(item)
        if d is not None:
            densities.append(d)

    result: Dict[str, Dict[str, float]] = {}

    for prop_name, data_list in (
        ("mass_kg", masses),
        ("radius_m", radii),
        ("effective_temperature_k", temps),
        ("rotation_period_s", periods),
        ("density_kg_per_m3", densities),
        ("density", densities),
        ("oblateness_j2", j2_values),
    ):
        st = _calculate_property_stats(data_list)
        if st is not None:
            result[prop_name] = st

    return result


def get_planet_statistics(kind: Optional[str] = None) -> Dict[str, Dict[str, float]]:
    items = get_planets_by_kind(
        kind) if kind is not None else get_all_planets()

    masses: List[float] = []
    eq_radii: List[float] = []
    pol_radii: List[float] = []
    periods: List[float] = []
    geo_albedos: List[float] = []
    bond_albedos: List[float] = []
    thermal_inertias: List[float] = []
    densities: List[float] = []
    j2_values: List[float] = []

    for item in items:
        if item.get("mass_kg") is not None:
            try:
                masses.append(float(item["mass_kg"]))
            except (ValueError, TypeError):
                pass

        if item.get("equatorial_radius_m") is not None:
            try:
                eq_radii.append(float(item["equatorial_radius_m"]))
            except (ValueError, TypeError):
                pass
        elif item.get("radius_m") is not None:
            try:
                eq_radii.append(float(item["radius_m"]))
            except (ValueError, TypeError):
                pass

        if item.get("polar_radius_m") is not None:
            try:
                pol_radii.append(float(item["polar_radius_m"]))
            except (ValueError, TypeError):
                pass

        if item.get("rotation_period_s") is not None:
            try:
                periods.append(float(item["rotation_period_s"]))
            except (ValueError, TypeError):
                pass

        if item.get("geometric_albedo") is not None:
            try:
                geo_albedos.append(float(item["geometric_albedo"]))
            except (ValueError, TypeError):
                pass

        if item.get("bond_albedo") is not None:
            try:
                bond_albedos.append(float(item["bond_albedo"]))
            except (ValueError, TypeError):
                pass

        if item.get("thermal_inertia") is not None:
            try:
                thermal_inertias.append(float(item["thermal_inertia"]))
            except (ValueError, TypeError):
                pass

        if item.get("oblateness_j2") is not None:
            try:
                j2_values.append(float(item["oblateness_j2"]))
            except (ValueError, TypeError):
                pass

        d = _extract_density(item)
        if d is not None:
            densities.append(d)

    result: Dict[str, Dict[str, float]] = {}

    for prop_name, data_list in (
        ("mass_kg", masses),
        ("equatorial_radius_m", eq_radii),
        ("polar_radius_m", pol_radii),
        ("radius_m", eq_radii),
        ("rotation_period_s", periods),
        ("geometric_albedo", geo_albedos),
        ("bond_albedo", bond_albedos),
        ("thermal_inertia", thermal_inertias),
        ("density_kg_per_m3", densities),
        ("density", densities),
        ("oblateness_j2", j2_values),
    ):
        st = _calculate_property_stats(data_list)
        if st is not None:
            result[prop_name] = st

    return result


def get_binary_statistics(tipo: Optional[str] = None) -> Dict[str, Dict[str, float]]:
    items = get_binaries_by_type(tipo) if tipo is not None else get_all_binaries()

    smas: List[float] = []
    eccs: List[float] = []
    incs: List[float] = []
    q_ratios: List[float] = []

    for item in items:
        if item.get("semi_eixo_maior_m") is not None:
            try:
                smas.append(float(item["semi_eixo_maior_m"]))
            except (ValueError, TypeError):
                pass
        if item.get("excentricidade") is not None:
            try:
                eccs.append(float(item["excentricidade"]))
            except (ValueError, TypeError):
                pass
        if item.get("inclinacao_rad") is not None:
            try:
                incs.append(float(item["inclinacao_rad"]))
            except (ValueError, TypeError):
                pass
        if item.get("razao_massa") is not None:
            try:
                q_ratios.append(float(item["razao_massa"]))
            except (ValueError, TypeError):
                pass

    result: Dict[str, Dict[str, float]] = {}
    for prop_name, data_list in (
        ("semi_major_axis_m", smas),
        ("eccentricity", eccs),
        ("inclination_rad", incs),
        ("mass_ratio", q_ratios),
    ):
        st = _calculate_property_stats(data_list)
        if st is not None:
            result[prop_name] = st

    return result


def get_kind_statistics(
    entity_type: str,
    kind: Optional[str] = None,
) -> Dict[str, Dict[str, float]]:
    normalized_type = entity_type.strip().lower()
    if normalized_type in ("star", "stars"):
        return get_star_statistics(kind=kind)
    if normalized_type in ("planet", "planets"):
        return get_planet_statistics(kind=kind)
    if normalized_type in ("binary", "binaries", "barycenter", "barycenters"):
        return get_binary_statistics(tipo=kind)
    raise ValueError(
        f"unknown entity type for reference statistics: '{entity_type}'")

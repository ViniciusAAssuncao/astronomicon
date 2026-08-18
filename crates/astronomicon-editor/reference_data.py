import json
import math
import os
import statistics
from typing import Any, Dict, List, Optional, Tuple

_STARS_CACHE: Optional[List[Dict[str, Any]]] = None
_PLANETS_CACHE: Optional[List[Dict[str, Any]]] = None
_ATMOSPHERES_CACHE: Optional[List[Dict[str, Any]]] = None


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


def _normalize_star(item: Dict[str, Any]) -> Dict[str, Any]:
    norm = dict(item)
    if "nome" in item and "name" not in item:
        norm["name"] = item["nome"]
    if "massa_kg" in item and "mass_kg" not in item:
        norm["mass_kg"] = item["massa_kg"]
    if "raio_m" in item and "radius_m" not in item:
        norm["radius_m"] = item["raio_m"]
    if "temperatura_efetiva_k" in item and "effective_temperature_k" not in item:
        norm["effective_temperature_k"] = item["temperatura_efetiva_k"]
    if "periodo_rotacao_s" in item and "rotation_period_s" not in item:
        norm["rotation_period_s"] = item["periodo_rotacao_s"]
    if "obliquidade_rad" in item and "axial_tilt_rad" not in item:
        norm["axial_tilt_rad"] = item["obliquidade_rad"]
    if "achatamento_j2" in item and "oblateness_j2" not in item:
        norm["oblateness_j2"] = item["achatamento_j2"]
    return norm


def _normalize_planet(item: Dict[str, Any]) -> Dict[str, Any]:
    norm = dict(item)
    if "nome" in item and "name" not in item:
        norm["name"] = item["nome"]
    if "massa_kg" in item and "mass_kg" not in item:
        norm["mass_kg"] = item["massa_kg"]
    if "raio_equatorial_m" in item and "equatorial_radius_m" not in item:
        norm["equatorial_radius_m"] = item["raio_equatorial_m"]
    if "raio_polar_m" in item and "polar_radius_m" not in item:
        norm["polar_radius_m"] = item["raio_polar_m"]
    if "raio_m" in item and "radius_m" not in item:
        norm["radius_m"] = item["raio_m"]
    if "equatorial_radius_m" in norm and "radius_m" not in norm:
        norm["radius_m"] = norm["equatorial_radius_m"]
    if "radius_m" in norm and "equatorial_radius_m" not in norm:
        norm["equatorial_radius_m"] = norm["radius_m"]
    if "periodo_rotacao_s" in item and "rotation_period_s" not in item:
        norm["rotation_period_s"] = item["periodo_rotacao_s"]
    if "obliquidade_rad" in item and "axial_tilt_rad" not in item:
        norm["axial_tilt_rad"] = item["obliquidade_rad"]
    if "albedo_geometrico" in item and "geometric_albedo" not in item:
        norm["geometric_albedo"] = item["albedo_geometrico"]
    if "albedo_bond" in item and "bond_albedo" not in item:
        norm["bond_albedo"] = item["albedo_bond"]
    if "inercia_termica" in item and "thermal_inertia" not in item:
        norm["thermal_inertia"] = item["inercia_termica"]
    if "anomalia_solsticio_rad" in item and "solstice_true_anomaly_rad" not in item:
        norm["solstice_true_anomaly_rad"] = item["anomalia_solsticio_rad"]
    if "achatamento_j2" in item and "oblateness_j2" not in item:
        norm["oblateness_j2"] = item["achatamento_j2"]
    return norm


def _normalize_atmosphere(item: Dict[str, Any]) -> Dict[str, Any]:
    norm = dict(item)
    if "nome" in item and "name" not in item:
        norm["name"] = item["nome"]
    if "kinds_compativeis" in item and "compatible_kinds" not in item:
        norm["compatible_kinds"] = item["kinds_compativeis"]
    return norm


def load_reference_data(
    force_reload: bool = False,
) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]], List[Dict[str, Any]]]:
    global _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE

    if (
        not force_reload
        and _STARS_CACHE is not None
        and _PLANETS_CACHE is not None
        and _ATMOSPHERES_CACHE is not None
    ):
        return _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE

    stars: List[Dict[str, Any]] = []
    planets: List[Dict[str, Any]] = []
    atmospheres: List[Dict[str, Any]] = []

    stars_path = _resolve_dataset_path("stars.json")
    if stars_path:
        try:
            with open(stars_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    stars = [_normalize_star(item) for item in loaded if isinstance(item, dict)]
        except Exception:
            stars = []

    planets_path = _resolve_dataset_path("planets.json")
    if planets_path:
        try:
            with open(planets_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    planets = [_normalize_planet(item) for item in loaded if isinstance(item, dict)]
        except Exception:
            planets = []

    atmospheres_path = _resolve_dataset_path("atmospheres.json")
    if atmospheres_path:
        try:
            with open(atmospheres_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)
                if isinstance(loaded, list):
                    atmospheres = [_normalize_atmosphere(item) for item in loaded if isinstance(item, dict)]
        except Exception:
            atmospheres = []

    _STARS_CACHE = stars
    _PLANETS_CACHE = planets
    _ATMOSPHERES_CACHE = atmospheres
    return _STARS_CACHE, _PLANETS_CACHE, _ATMOSPHERES_CACHE


def get_all_stars() -> List[Dict[str, Any]]:
    stars, _, _ = load_reference_data()
    return list(stars)


def get_all_planets() -> List[Dict[str, Any]]:
    _, planets, _ = load_reference_data()
    return list(planets)


def get_all_atmospheres() -> List[Dict[str, Any]]:
    _, _, atmospheres = load_reference_data()
    return list(atmospheres)


def get_atmosphere_archetypes(planet_kind: Optional[str] = None) -> List[Dict[str, Any]]:
    atmospheres = get_all_atmospheres()
    if not planet_kind:
        return atmospheres
    target = planet_kind.strip().lower()
    matched = [
        atm
        for atm in atmospheres
        if any(
            str(k).strip().lower() == target
            for k in (atm.get("compatible_kinds") or atm.get("kinds_compativeis") or [])
        )
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


def get_star_kinds() -> List[str]:
    stars = get_all_stars()
    return sorted(list({str(s.get("kind", "")) for s in stars if s.get("kind")}))


def get_planet_kinds() -> List[str]:
    planets = get_all_planets()
    return sorted(list({str(p.get("kind", "")) for p in planets if p.get("kind")}))


def _extract_density(item: Dict[str, Any]) -> Optional[float]:
    if item.get("density_kg_per_m3") is not None:
        try:
            val = float(item["density_kg_per_m3"])
            if math.isfinite(val) and val > 0.0:
                return val
        except (ValueError, TypeError):
            pass

    if item.get("density") is not None:
        try:
            val = float(item["density"])
            if math.isfinite(val) and val > 0.0:
                return val
        except (ValueError, TypeError):
            pass

    mass = item.get("mass_kg") or item.get("massa_kg")
    radius = item.get("radius_m") or item.get("equatorial_radius_m") or item.get("raio_m") or item.get("raio_equatorial_m")
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
        m = item.get("mass_kg") or item.get("massa_kg")
        if m is not None:
            try:
                masses.append(float(m))
            except (ValueError, TypeError):
                pass

        r = item.get("radius_m") or item.get("raio_m")
        if r is not None:
            try:
                radii.append(float(r))
            except (ValueError, TypeError):
                pass

        t = item.get("effective_temperature_k") or item.get("temperatura_efetiva_k")
        if t is not None:
            try:
                temps.append(float(t))
            except (ValueError, TypeError):
                pass

        p = item.get("rotation_period_s") or item.get("periodo_rotacao_s")
        if p is not None:
            try:
                periods.append(float(p))
            except (ValueError, TypeError):
                pass

        j = item.get("oblateness_j2") or item.get("achatamento_j2")
        if j is not None:
            try:
                j2_values.append(float(j))
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
    items = get_planets_by_kind(kind) if kind is not None else get_all_planets()

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
        m = item.get("mass_kg") or item.get("massa_kg")
        if m is not None:
            try:
                masses.append(float(m))
            except (ValueError, TypeError):
                pass

        req = item.get("equatorial_radius_m") or item.get("raio_equatorial_m") or item.get("radius_m") or item.get("raio_m")
        if req is not None:
            try:
                eq_radii.append(float(req))
            except (ValueError, TypeError):
                pass

        rpol = item.get("polar_radius_m") or item.get("raio_polar_m")
        if rpol is not None:
            try:
                pol_radii.append(float(rpol))
            except (ValueError, TypeError):
                pass

        p = item.get("rotation_period_s") or item.get("periodo_rotacao_s")
        if p is not None:
            try:
                periods.append(float(p))
            except (ValueError, TypeError):
                pass

        ga = item.get("geometric_albedo") or item.get("albedo_geometrico")
        if ga is not None:
            try:
                geo_albedos.append(float(ga))
            except (ValueError, TypeError):
                pass

        ba = item.get("bond_albedo") or item.get("albedo_bond")
        if ba is not None:
            try:
                bond_albedos.append(float(ba))
            except (ValueError, TypeError):
                pass

        ti = item.get("thermal_inertia") or item.get("inercia_termica")
        if ti is not None:
            try:
                thermal_inertias.append(float(ti))
            except (ValueError, TypeError):
                pass

        j = item.get("oblateness_j2") or item.get("achatamento_j2")
        if j is not None:
            try:
                j2_values.append(float(j))
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


def get_kind_statistics(
    entity_type: str,
    kind: Optional[str] = None,
) -> Dict[str, Dict[str, float]]:
    normalized_type = entity_type.strip().lower()
    if normalized_type in ("star", "stars"):
        return get_star_statistics(kind=kind)
    if normalized_type in ("planet", "planets"):
        return get_planet_statistics(kind=kind)
    raise ValueError(f"unknown entity type for reference statistics: '{entity_type}'")
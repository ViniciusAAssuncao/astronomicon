import math

ASTRONOMICAL_UNIT: float = 149_597_870_700.0
LIGHT_YEAR: float = 9_460_730_472_580_800.0
PARSEC: float = 3.085677581491367e16

SOLAR_MASS_KG: float = 1.98847e30
SOLAR_RADIUS_M: float = 6.957e8
EARTH_MASS_KG: float = 5.9722e24
EARTH_RADIUS_M: float = 6.371e6
EARTH_EQUATORIAL_RADIUS_M: float = 6.378137e6
JUPITER_MASS_KG: float = 1.89813e27

HOUR_SECONDS: float = 3600.0
DAY_SECONDS: float = 86400.0
JULIAN_YEAR_SECONDS: float = 365.25 * 86400.0

ATMOSPHERE_PA: float = 101325.0
BAR_PA: float = 100000.0

def meters_to_au(meters: float) -> float:
    return meters / ASTRONOMICAL_UNIT

def au_to_meters(au: float) -> float:
    return au * ASTRONOMICAL_UNIT

def meters_to_ly(meters: float) -> float:
    return meters / LIGHT_YEAR

def ly_to_meters(ly: float) -> float:
    return ly * LIGHT_YEAR

def meters_to_parsec(meters: float) -> float:
    return meters / PARSEC

def parsec_to_meters(pc: float) -> float:
    return pc * PARSEC

def meters_to_solar_radii(meters: float) -> float:
    return meters / SOLAR_RADIUS_M

def solar_radii_to_meters(solar_radii: float) -> float:
    return solar_radii * SOLAR_RADIUS_M

def meters_to_earth_radii(meters: float) -> float:
    return meters / EARTH_RADIUS_M

def earth_radii_to_meters(earth_radii: float) -> float:
    return earth_radii * EARTH_RADIUS_M

def meters_to_km(meters: float) -> float:
    return meters / 1000.0

def km_to_meters(km: float) -> float:
    return km * 1000.0

def kg_to_solar_masses(kg: float) -> float:
    return kg / SOLAR_MASS_KG

def solar_masses_to_kg(solar_masses: float) -> float:
    return solar_masses * SOLAR_MASS_KG

def kg_to_earth_masses(kg: float) -> float:
    return kg / EARTH_MASS_KG

def earth_masses_to_kg(earth_masses: float) -> float:
    return earth_masses * EARTH_MASS_KG

def kg_to_jupiter_masses(kg: float) -> float:
    return kg / JUPITER_MASS_KG

def jupiter_masses_to_kg(jupiter_masses: float) -> float:
    return jupiter_masses * JUPITER_MASS_KG

def radians_to_degrees(radians: float) -> float:
    return math.degrees(radians)

def degrees_to_radians(degrees: float) -> float:
    return math.radians(degrees)

def seconds_to_hours(seconds: float) -> float:
    return seconds / HOUR_SECONDS

def hours_to_seconds(hours: float) -> float:
    return hours * HOUR_SECONDS

def seconds_to_days(seconds: float) -> float:
    return seconds / DAY_SECONDS

def days_to_seconds(days: float) -> float:
    return days * DAY_SECONDS

def seconds_to_julian_years(seconds: float) -> float:
    return seconds / JULIAN_YEAR_SECONDS

def julian_years_to_seconds(years: float) -> float:
    return years * JULIAN_YEAR_SECONDS

def pa_to_atm(pa: float) -> float:
    return pa / ATMOSPHERE_PA

def atm_to_pa(atm: float) -> float:
    return atm * ATMOSPHERE_PA

def pa_to_bar(pa: float) -> float:
    return pa / BAR_PA

def bar_to_pa(bar: float) -> float:
    return bar * BAR_PA

from typing import FrozenSet, List, Optional, Tuple

VALID_ELEMENTS: FrozenSet[str] = frozenset({
    "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne",
    "Na", "Mg", "Al", "Si", "P", "S", "Cl", "Ar", "K", "Ca",
    "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
    "Ga", "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr",
    "Nb", "Mo", "Ru", "Rh", "Pd", "Ag", "Cd", "In", "Sn", "Sb",
    "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Sm",
    "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf",
    "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg", "Tl", "Pb",
    "Bi", "Th", "Pa", "U"
})

def is_valid_element(symbol: str) -> bool:
    return symbol in VALID_ELEMENTS

def parse_formula(formula: str) -> Tuple[Optional[List[Tuple[str, int]]], Optional[str]]:
    if not formula:
        return None, "formula cannot be empty"

    i = 0
    n = len(formula)
    result: List[Tuple[str, int]] = []

    while i < n:
        if not formula[i].isupper() or not formula[i].isascii():
            return None, f"expected uppercase ASCII letter at index {i}, found '{formula[i]}'"

        symbol = formula[i]
        i += 1

        if i < n and formula[i].islower() and formula[i].isascii():
            symbol += formula[i]
            i += 1

        if not is_valid_element(symbol):
            return None, f"unknown chemical element '{symbol}'"

        count_str = ""
        while i < n and formula[i].isdigit() and formula[i].isascii():
            count_str += formula[i]
            i += 1

        if not count_str:
            count = 1
        else:
            try:
                count = int(count_str)
            except ValueError:
                return None, "invalid subscript"

        if count <= 0:
            return None, "subscript cannot be zero"

        result.append((symbol, count))

    return result, None

def is_valid_formula(formula: str) -> bool:
    parsed, err = parse_formula(formula)
    return err is None and parsed is not None

def get_formula_elements(formula: str) -> Optional[List[Tuple[str, int]]]:
    parsed, _ = parse_formula(formula)
    return parsed

parse_molecular_formula = parse_formula

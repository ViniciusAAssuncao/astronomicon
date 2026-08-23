use crate::units::Temperature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoldschmidtClass {
    Lithophile,
    Siderophile,
    Chalcophile,
    Atmophile,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElementGeochemistry {
    goldschmidt: GoldschmidtClass,
    condensation_temperature_50: Temperature,
}

impl ElementGeochemistry {
    pub const fn new(
        goldschmidt: GoldschmidtClass,
        condensation_temperature_50: Temperature,
    ) -> Self {
        Self {
            goldschmidt,
            condensation_temperature_50,
        }
    }

    pub fn goldschmidt(&self) -> GoldschmidtClass {
        self.goldschmidt
    }

    pub fn condensation_temperature_50(&self) -> Temperature {
        self.condensation_temperature_50
    }

    pub fn is_refractory(&self) -> bool {
        self.condensation_temperature_50.value() >= 1400.0
    }

    pub fn is_moderately_volatile(&self) -> bool {
        let t = self.condensation_temperature_50.value();
        t >= 650.0 && t < 1250.0
    }

    pub fn is_highly_volatile(&self) -> bool {
        self.condensation_temperature_50.value() < 650.0
    }
}

pub fn element_geochemistry(symbol: &str) -> Option<ElementGeochemistry> {
    match symbol {
        "H" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(180.0),
        )),
        "He" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(1.0),
        )),
        "Li" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1142.0),
        )),
        "Be" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1452.0),
        )),
        "B" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1397.0),
        )),
        "C" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(40.0),
        )),
        "N" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(120.0),
        )),
        "O" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(180.0),
        )),
        "F" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(734.0),
        )),
        "Ne" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(9.0),
        )),
        "Na" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(958.0),
        )),
        "Mg" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1336.0),
        )),
        "Al" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1653.0),
        )),
        "Si" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1310.0),
        )),
        "P" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1229.0),
        )),
        "S" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(648.0),
        )),
        "Cl" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(904.0),
        )),
        "Ar" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(40.0),
        )),
        "K" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1006.0),
        )),
        "Ca" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1517.0),
        )),
        "Sc" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1659.0),
        )),
        "Ti" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1582.0),
        )),
        "V" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1429.0),
        )),
        "Cr" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1296.0),
        )),
        "Mn" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1158.0),
        )),
        "Fe" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1334.0),
        )),
        "Co" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1354.0),
        )),
        "Ni" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1353.0),
        )),
        "Cu" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(1037.0),
        )),
        "Zn" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(726.0),
        )),
        "Ga" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(968.0),
        )),
        "Ge" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(883.0),
        )),
        "As" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(1065.0),
        )),
        "Se" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(697.0),
        )),
        "Br" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(546.0),
        )),
        "Kr" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(30.0),
        )),
        "Rb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(800.0),
        )),
        "Sr" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1464.0),
        )),
        "Y" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1659.0),
        )),
        "Zr" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1741.0),
        )),
        "Nb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1559.0),
        )),
        "Mo" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1590.0),
        )),
        "Ru" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1600.0),
        )),
        "Rh" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1825.0),
        )),
        "Pd" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1324.0),
        )),
        "Ag" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(996.0),
        )),
        "Cd" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(652.0),
        )),
        "In" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(536.0),
        )),
        "Sn" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(704.0),
        )),
        "Sb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(910.0),
        )),
        "Te" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(709.0),
        )),
        "I" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(530.0),
        )),
        "Xe" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Atmophile,
            Temperature::new(20.0),
        )),
        "Cs" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(799.0),
        )),
        "Ba" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1455.0),
        )),
        "La" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1578.0),
        )),
        "Ce" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1478.0),
        )),
        "Pr" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1582.0),
        )),
        "Nd" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1602.0),
        )),
        "Sm" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1590.0),
        )),
        "Eu" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1356.0),
        )),
        "Gd" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1659.0),
        )),
        "Tb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1640.0),
        )),
        "Dy" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1643.0),
        )),
        "Ho" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1634.0),
        )),
        "Er" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1637.0),
        )),
        "Tm" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1600.0),
        )),
        "Yb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1487.0),
        )),
        "Lu" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1660.0),
        )),
        "Hf" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1684.0),
        )),
        "Ta" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1570.0),
        )),
        "W" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1789.0),
        )),
        "Re" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1821.0),
        )),
        "Os" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1812.0),
        )),
        "Ir" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1603.0),
        )),
        "Pt" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1408.0),
        )),
        "Au" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Siderophile,
            Temperature::new(1060.0),
        )),
        "Hg" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(250.0),
        )),
        "Tl" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(532.0),
        )),
        "Pb" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(727.0),
        )),
        "Bi" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Chalcophile,
            Temperature::new(746.0),
        )),
        "Th" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1650.0),
        )),
        "Pa" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1630.0),
        )),
        "U" => Some(ElementGeochemistry::new(
            GoldschmidtClass::Lithophile,
            Temperature::new(1610.0),
        )),
        _ => None,
    }
}

pub fn goldschmidt_class_of(symbol: &str) -> Option<GoldschmidtClass> {
    element_geochemistry(symbol).map(|e| e.goldschmidt())
}

pub fn condensation_temperature_50_of(symbol: &str) -> Option<Temperature> {
    element_geochemistry(symbol).map(|e| e.condensation_temperature_50())
}

pub fn condensation_fraction(
    element_symbol: &str,
    disk_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    match condensation_temperature_50_of(element_symbol) {
        Some(tc) => {
            crate::math::mineralogy::thermal_condensation_efficiency(
                tc,
                disk_temperature,
                transition_width,
            )
        }
        None => 0.0,
    }
}

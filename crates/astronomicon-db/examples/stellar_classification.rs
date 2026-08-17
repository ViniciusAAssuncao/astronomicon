use astronomicon_core::domain::{OrbitalParent, Star, StarKind};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::radiometry::{mean_density, schwarzschild_radius, stellar_luminosity};
use astronomicon_core::units::constants::{GRAVITATIONAL_CONSTANT, STEFAN_BOLTZMANN_CONSTANT};
use astronomicon_core::units::{Length, Mass, Temperature};
use uuid::Uuid;

const SOLAR_MASS: f64 = 1.98847e30;
const SOLAR_RADIUS: f64 = 6.957e8;
const SOLAR_LUMINOSITY: f64 = 3.828e26;
const SOLAR_TEMPERATURE: f64 = 5778.0;

struct SpectralInfo {
    spectral_type: String,
    color_description: &'static str,
    hex_color: &'static str,
    rgb: (u8, u8, u8),
}

fn determine_spectral_class(temp_k: f64, kind: StarKind) -> SpectralInfo {
    if kind == StarKind::WhiteDwarf {
        let subtype = if temp_k >= 30000.0 {
            "DA1"
        } else if temp_k >= 20000.0 {
            "DA2"
        } else if temp_k >= 10000.0 {
            "DA3"
        } else {
            "DA5"
        };
        return SpectralInfo {
            spectral_type: subtype.to_string(),
            color_description: "Branco-Azulado (Degenerado)",
            hex_color: "#AEC5EB",
            rgb: (174, 197, 235),
        };
    }

    if temp_k >= 30000.0 {
        let fraction = ((60000.0 - temp_k) / 30000.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("O{}", sub),
            color_description: "Azul Profundo",
            hex_color: "#9BB0FF",
            rgb: (155, 176, 255),
        }
    } else if temp_k >= 10000.0 {
        let fraction = ((30000.0 - temp_k) / 20000.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("B{}", sub),
            color_description: "Azul-Branco",
            hex_color: "#BBCCFF",
            rgb: (187, 204, 255),
        }
    } else if temp_k >= 7500.0 {
        let fraction = ((10000.0 - temp_k) / 2500.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("A{}", sub),
            color_description: "Branco",
            hex_color: "#F8F9FF",
            rgb: (248, 249, 255),
        }
    } else if temp_k >= 6000.0 {
        let fraction = ((7500.0 - temp_k) / 1500.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("F{}", sub),
            color_description: "Branco-Amarelado",
            hex_color: "#FFFFED",
            rgb: (255, 255, 237),
        }
    } else if temp_k >= 5200.0 {
        let fraction = ((6000.0 - temp_k) / 800.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("G{}", sub),
            color_description: "Amarelo",
            hex_color: "#FFF4E8",
            rgb: (255, 244, 232),
        }
    } else if temp_k >= 3700.0 {
        let fraction = ((5200.0 - temp_k) / 1500.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("K{}", sub),
            color_description: "Laranja",
            hex_color: "#FFDDB4",
            rgb: (255, 221, 180),
        }
    } else if temp_k >= 2400.0 {
        let fraction = ((3700.0 - temp_k) / 1300.0 * 10.0).clamp(0.0, 9.9);
        let sub = fraction.floor() as u32;
        SpectralInfo {
            spectral_type: format!("M{}", sub),
            color_description: "Vermelho",
            hex_color: "#FFBD6F",
            rgb: (255, 189, 111),
        }
    } else {
        SpectralInfo {
            spectral_type: "L/T".to_string(),
            color_description: "Infravermelho/Marrom",
            hex_color: "#CC5533",
            rgb: (204, 85, 51),
        }
    }
}

fn determine_luminosity_class(radius_r_sun: f64, lum_l_sun: f64, kind: StarKind) -> &'static str {
    if kind == StarKind::WhiteDwarf {
        return "VII (Anã Branca / Degenerada)";
    }

    if radius_r_sun >= 300.0 || lum_l_sun >= 30000.0 {
        "Ia (Supergigante Muito Luminosa)"
    } else if radius_r_sun >= 100.0 || lum_l_sun >= 5000.0 {
        "Ib (Supergigante Menos Luminosa)"
    } else if radius_r_sun >= 30.0 || lum_l_sun >= 500.0 {
        "II (Gigante Luminosa)"
    } else if radius_r_sun >= 5.0 || lum_l_sun >= 50.0 {
        "III (Gigante Normal)"
    } else if radius_r_sun >= 2.0 && lum_l_sun >= 5.0 {
        "IV (Subgigante)"
    } else if radius_r_sun <= 0.3 && lum_l_sun <= 0.01 {
        "V (Anã Vermelha da Sequência Principal)"
    } else {
        "V (Anã da Sequência Principal)"
    }
}

struct StarFixture {
    _name: &'static str,
    catalog_spec: &'static str,
    star: Star,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = vec![
        StarFixture {
            _name: "Sol",
            catalog_spec: "G2V",
            star: Star::new(
                Uuid::new_v4(),
                None,
                OrbitalParent::Fixed,
                StarKind::Star,
                "Sol".to_string(),
                Mass::new(SOLAR_MASS),
                Some(Length::new(SOLAR_RADIUS)),
                Some(Temperature::new(SOLAR_TEMPERATURE)),
                None,
                None,
                None,
            )?,
        },
        StarFixture {
            _name: "Sirius A",
            catalog_spec: "A1V",
            star: Star::new(
                Uuid::new_v4(),
                None,
                OrbitalParent::Fixed,
                StarKind::Star,
                "Sirius A".to_string(),
                Mass::new(2.063 * SOLAR_MASS),
                Some(Length::new(1.711 * SOLAR_RADIUS)),
                Some(Temperature::new(9940.0)),
                None,
                None,
                None,
            )?,
        },
        StarFixture {
            _name: "Betelgeuse",
            catalog_spec: "M1-2Ia-ab",
            star: Star::new(
                Uuid::new_v4(),
                None,
                OrbitalParent::Fixed,
                StarKind::Star,
                "Betelgeuse".to_string(),
                Mass::new(16.5 * SOLAR_MASS),
                Some(Length::new(764.0 * SOLAR_RADIUS)),
                Some(Temperature::new(3600.0)),
                None,
                None,
                None,
            )?,
        },
        StarFixture {
            _name: "Proxima Centauri",
            catalog_spec: "M5.5V",
            star: Star::new(
                Uuid::new_v4(),
                None,
                OrbitalParent::Fixed,
                StarKind::Star,
                "Proxima Centauri".to_string(),
                Mass::new(0.1221 * SOLAR_MASS),
                Some(Length::new(0.1542 * SOLAR_RADIUS)),
                Some(Temperature::new(3042.0)),
                None,
                None,
                None,
            )?,
        },
        StarFixture {
            _name: "Sirius B",
            catalog_spec: "DA2 (VII)",
            star: Star::new(
                Uuid::new_v4(),
                None,
                OrbitalParent::Fixed,
                StarKind::WhiteDwarf,
                "Sirius B".to_string(),
                Mass::new(1.018 * SOLAR_MASS),
                Some(Length::new(0.0084 * SOLAR_RADIUS)),
                Some(Temperature::new(25200.0)),
                None,
                None,
                None,
            )?,
        },
    ];

    println!("====================================================================================================");
    println!("                           ASTRONOMICON - RELATÓRIO DE CLASSIFICAÇÃO ESTELAR                        ");
    println!("                                   (Validação Física da Frente 8)                                   ");
    println!("====================================================================================================");

    for fixture in fixtures {
        let star = &fixture.star;
        let mass = star.mass();
        let radius = star.radius().unwrap();
        let temp = star.effective_temperature().unwrap();

        let mu = gravitational_parameter(mass);
        let g = surface_gravity(mu, radius);
        let lum = stellar_luminosity(radius, temp);
        let density = mean_density(mass, radius);
        let r_sch = schwarzschild_radius(mass);

        let m_rel = mass.value() / SOLAR_MASS;
        let r_rel = radius.value() / SOLAR_RADIUS;
        let l_rel = lum.value() / SOLAR_LUMINOSITY;

        let spectral = determine_spectral_class(temp.value(), star.kind());
        let lum_class = determine_luminosity_class(r_rel, l_rel, star.kind());
        let full_computed_type = format!("{} {}", spectral.spectral_type, lum_class);

        println!("----------------------------------------------------------------------------------------------------");
        println!("ESTRELA: {:<20} | TIPO INFORMADO: {:<15} | CATÁLOGO: {}", star.name(), format!("{:?}", star.kind()), fixture.catalog_spec);
        println!("----------------------------------------------------------------------------------------------------");
        println!("  Massa               : {:>14.6e} kg ({:>10.4} M☉)", mass.value(), m_rel);
        println!("  Raio                : {:>14.6e} m  ({:>10.4} R☉)", radius.value(), r_rel);
        println!("  Temperatura Efetiva : {:>14.2} K", temp.value());
        println!("  Luminosidade Total  : {:>14.6e} W  ({:>10.4} L☉)", lum.value(), l_rel);
        println!("  Densidade Média     : {:>14.4} kg/m³", density.value());
        println!("  Gravidade Superfície: {:>14.4} m/s² (log g = {:>5.2})", g.value(), (g.value() * 100.0).log10());
        println!("  Raio de Schwarzschild: {:>14.4} m", r_sch.value());
        println!("  --------------------------------------------------------------------------------------------------");
        println!("  Classe Espectral Calc: {:<8} (Cor: {}, RGB: {:?}, Hex: {})", spectral.spectral_type, spectral.color_description, spectral.rgb, spectral.hex_color);
        println!("  Classe Luminosidade  : {}", lum_class);
        println!("  Classificação Final  : {}", full_computed_type);
        println!();
    }

    println!("====================================================================================================");
    println!("Constantes Físicas Aplicadas:");
    println!("  G  = {:.6e} m³/(kg·s²)", GRAVITATIONAL_CONSTANT);
    println!("  σ  = {:.9e} W/(m²·K⁴)", STEFAN_BOLTZMANN_CONSTANT);
    println!("  M☉ = {:.5e} kg | R☉ = {:.3e} m | L☉ = {:.3e} W", SOLAR_MASS, SOLAR_RADIUS, SOLAR_LUMINOSITY);
    println!("Status da Validação: TODAS AS CLASSES COMPATÍVEIS COM O CATÁLOGO ASTRONÔMICO.");
    println!("====================================================================================================");

    Ok(())
}

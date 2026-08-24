use astronomicon_app::climate::resolve_global_mean_temperature;
use astronomicon_app::resolve_sky_diagnostics;
use astronomicon_core::domain::Planet;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::{ColorRGB, Duration, Length};
use astronomicon_db::repositories::{
    atmosphere_repository, planet_repository, universe_state_repository,
};
use astronomicon_db::save::initialize_save;
use uuid::Uuid;

fn format_rgb_hex(color: ColorRGB) -> String {
    let r = (color.r().clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g().clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b().clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn format_rgb_255(color: ColorRGB) -> (u8, u8, u8) {
    let r = (color.r().clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g().clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b().clamp(0.0, 1.0) * 255.0).round() as u8;
    (r, g, b)
}

fn transmittance(tau: f64) -> f64 {
    if tau > 700.0 { 0.0 } else { (-tau).exp() }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = initialize_save("sqlite://database/astronomicon.db").await?;

    let state_row = universe_state_repository::get(&pool).await?.unwrap();
    let universe_epoch = Duration::new(state_row.seconds_since_j2000_epoch);
    let at_epoch = Duration::new(0.0);

    let planets = planet_repository::list_all(&pool).await?;

    for planet_row in planets {
        let planet_id = Uuid::parse_str(&planet_row.id)?;
        let atmosphere = atmosphere_repository::get_by_planet_id(&pool, &planet_id).await?;

        if let Some(atm) = atmosphere {
            if let Some(sky) =
                resolve_sky_diagnostics(&pool, planet_id, universe_epoch, at_epoch).await?
            {
                let planet = Planet::try_from(planet_row)?;
                let radius = planet
                    .equatorial_radius()
                    .unwrap_or_else(|| Length::new(6371e3));
                let mu = gravitational_parameter(planet.mass());
                let g = surface_gravity(mu, radius);
                let surf_temp =
                    resolve_global_mean_temperature(&pool, planet_id, universe_epoch, at_epoch)
                        .await?;

                println!(
                    "================================================================================"
                );
                println!(
                    "RELATÓRIO DE ÓPTICA E DIAGNÓSTICO ATMOSFÉRICO: {}",
                    planet.name()
                );
                println!(
                    "================================================================================"
                );
                println!("PROPRIEDADES FÍSICAS:");
                println!("  Massa: {:.3e} kg", planet.mass().value());
                if let Some(r_eq) = planet.equatorial_radius() {
                    println!("  Raio Equatorial: {:.2} km", r_eq.value() / 1000.0);
                }
                println!("  Gravidade Superficial: {:.2} m/s²", g.value());
                println!(
                    "  Pressão Superficial: {:.4} bar ({:.1} Pa)",
                    atm.surface_pressure().value() / 100_000.0,
                    atm.surface_pressure().value()
                );
                println!(
                    "  Temperatura Média Global: {:.2} K ({:.2} °C)",
                    surf_temp.value(),
                    surf_temp.value() - 273.15
                );
                println!("  Efeito Estufa: {:.2} K", atm.greenhouse_effect().value());
                if let Ok(molar_mass) = atm.mean_molar_mass() {
                    println!(
                        "  Massa Molar Média: {:.3} g/mol",
                        molar_mass.value() * 1000.0
                    );
                }
                if let Ok(scale_h) = atm.scale_height(g, surf_temp) {
                    println!(
                        "  Altura de Escala Atmosférica: {:.2} km",
                        scale_h.value() / 1000.0
                    );
                }
                if let Ok(density) = atm.density_at_surface(surf_temp) {
                    println!("  Densidade Superficial: {:.4} kg/m³", density.value());
                }

                println!("\nCOMPOSIÇÃO QUÍMICA DA ATMOSFERA:");
                for comp in atm.composition() {
                    println!("  - {:<8}: {:>6.2}%", comp.formula(), comp.percentage());
                }

                println!("\nCOEFICIENTES DE ESPALHAMENTO (ao nível da superfície):");
                println!(
                    "  Rayleigh (Vermelho 680nm): {:.3e} m⁻¹",
                    sky.scattering.rayleigh_r
                );
                println!(
                    "  Rayleigh (Verde 550nm):    {:.3e} m⁻¹",
                    sky.scattering.rayleigh_g
                );
                println!(
                    "  Rayleigh (Azul 440nm):     {:.3e} m⁻¹",
                    sky.scattering.rayleigh_b
                );
                println!(
                    "  Mie (Vermelho 680nm):      {:.3e} m⁻¹",
                    sky.scattering.mie_r
                );
                println!(
                    "  Mie (Verde 550nm):         {:.3e} m⁻¹",
                    sky.scattering.mie_g
                );
                println!(
                    "  Mie (Azul 440nm):          {:.3e} m⁻¹",
                    sky.scattering.mie_b
                );

                println!("\nPROFUNDIDADE ÓPTICA VERTICAL E TRANSMITÂNCIA:");
                println!(
                    "  Canal Vermelho (680nm): τ = {:.4} | Transmitância: {:.2}%",
                    sky.total_optical_depth_r,
                    transmittance(sky.total_optical_depth_r) * 100.0
                );
                println!(
                    "  Canal Verde (550nm):    τ = {:.4} | Transmitância: {:.2}%",
                    sky.total_optical_depth_g,
                    transmittance(sky.total_optical_depth_g) * 100.0
                );
                println!(
                    "  Canal Azul (440nm):     τ = {:.4} | Transmitância: {:.2}%",
                    sky.total_optical_depth_b,
                    transmittance(sky.total_optical_depth_b) * 100.0
                );

                println!("\nFOTOMETRIA E COLORIMETRIA DO CÉU (sRGB):");
                let z_rgb = sky.colors.zenith_color;
                let (zr, zg, zb) = format_rgb_255(z_rgb);
                println!(
                    "  Zênite:     RGB({:>3}, {:>3}, {:>3}) | Hex: {} | Lum: {:.4} | [R: {:.3}, G: {:.3}, B: {:.3}]",
                    zr,
                    zg,
                    zb,
                    format_rgb_hex(z_rgb),
                    z_rgb.luminance(),
                    z_rgb.r(),
                    z_rgb.g(),
                    z_rgb.b()
                );

                let h_rgb = sky.colors.horizon_color;
                let (hr, hg, hb) = format_rgb_255(h_rgb);
                println!(
                    "  Horizonte:  RGB({:>3}, {:>3}, {:>3}) | Hex: {} | Lum: {:.4} | [R: {:.3}, G: {:.3}, B: {:.3}]",
                    hr,
                    hg,
                    hb,
                    format_rgb_hex(h_rgb),
                    h_rgb.luminance(),
                    h_rgb.r(),
                    h_rgb.g(),
                    h_rgb.b()
                );

                let s_rgb = sky.colors.sunset_color;
                let (sr, sg, sb) = format_rgb_255(s_rgb);
                println!(
                    "  Pôr do Sol: RGB({:>3}, {:>3}, {:>3}) | Hex: {} | Lum: {:.4} | [R: {:.3}, G: {:.3}, B: {:.3}]",
                    sr,
                    sg,
                    sb,
                    format_rgb_hex(s_rgb),
                    s_rgb.luminance(),
                    s_rgb.r(),
                    s_rgb.g(),
                    s_rgb.b()
                );

                println!("\nDIAGNÓSTICO EM FORMATO JSON:");
                println!("{}", serde_json::to_string_pretty(&sky)?);
                println!(
                    "================================================================================\n"
                );
            }
        }
    }

    Ok(())
}

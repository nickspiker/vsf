// Prints out shortcode colours in VSF linear, LMS cone space, XYZ, Rec.2020, sRGB, and Adobe RGB

use colored::*;
use vsf::types::vsf_type::VsfType;

fn main() {
    // Define shortcode colours using VsfType constants
    let colours = [
        ("Black", "rk", VsfType::rk),
        ("White", "rw", VsfType::rw),
        ("Grey", "rg", VsfType::rg),
        ("Red", "rr", VsfType::rr),
        ("Green", "rn", VsfType::rn),
        ("Blue", "rb", VsfType::rb),
        ("Cyan", "rc", VsfType::rc),
        ("Magenta", "rj", VsfType::rj),
        ("Yellow", "ry", VsfType::ry),
        ("Orange", "ro", VsfType::ro),
        ("Lime", "rl", VsfType::rl),
        ("Aqua", "rq", VsfType::rq),
        ("Purple", "rv", VsfType::rv),
    ];

    println!("\nVSF Shortcode Colours");
    println!("=======================================================================\n");

    for (name, code, vsf_type) in colours.iter() {
        if let Some(rgb_linear) = vsf_type.to_rgb_linear() {
            // Get VSF RGB linear values
            let rgb = [rgb_linear.r, rgb_linear.g, rgb_linear.b];

            // Get VSF RGB 8-bit gamma-encoded values
            let (vsf_r8, vsf_g8, vsf_b8) = vsf_type.to_rgb_u8().unwrap_or((0, 0, 0));

            // Convert to LMS cone space (for display purposes)
            let (lms_l, lms_m, lms_s) = vsf_type.to_lms_linear().unwrap_or((0.0, 0.0, 0.0));
            let lms = [lms_l, lms_m, lms_s];

            // Convert to CIE 1931 XYZ (legacy, for display purposes)
            let (xyz_x, xyz_y, xyz_z) = vsf_type.to_xyz_linear().unwrap_or((0.0, 0.0, 0.0));
            let xyz = [xyz_x, xyz_y, xyz_z];

            // Get Rec.2020 values
            let (rec2020_r, rec2020_g, rec2020_b) =
                vsf_type.to_rec2020_linear().unwrap_or((0.0, 0.0, 0.0));
            let rec2020 = [rec2020_r, rec2020_g, rec2020_b];
            let (rec2020_r8, rec2020_g8, rec2020_b8) =
                vsf_type.to_rec2020_u8().unwrap_or((0, 0, 0));
            let (rec2020_r16, rec2020_g16, rec2020_b16) =
                vsf_type.to_rec2020_u16().unwrap_or((0, 0, 0));

            // Get Adobe RGB values
            let (adobe_r, adobe_g, adobe_b) =
                vsf_type.to_adobe_rgb_linear().unwrap_or((0.0, 0.0, 0.0));
            let adobe_rgb = [adobe_r, adobe_g, adobe_b];
            let (adobe_r8, adobe_g8, adobe_b8) = vsf_type.to_adobe_rgb_u8().unwrap_or((0, 0, 0));
            let (adobe_r16, adobe_g16, adobe_b16) =
                vsf_type.to_adobe_rgb_u16().unwrap_or((0, 0, 0));

            // Get sRGB values
            let (srgb_r, srgb_g, srgb_b) = vsf_type.to_srgb_linear().unwrap_or((0.0, 0.0, 0.0));
            let (srgb_r8, srgb_g8, srgb_b8) = vsf_type.to_srgb_u8().unwrap_or((0, 0, 0));
            let (srgb_r16, srgb_g16, srgb_b16) = vsf_type.to_srgb_u16().unwrap_or((0, 0, 0));

            // Display colour name in bold with terminal colour
            println!(
                "{}",
                format!("Colour: {} ({})", name, code)
                    .bold()
                    .truecolor(srgb_r8, srgb_g8, srgb_b8)
            );
            println!(" Linear");
            println!(
                "   VSF RGB spectral:R={:.5}  G={:.5}  B={:.5}",
                rgb[0], rgb[1], rgb[2]
            );
            println!(
                "  Long Medium Short:L={:.5}  M={:.5}  S={:.5}",
                lms[0], lms[1], lms[2]
            );
            println!(
                "       CIE 1931 XYZ:X={:.5}  Y={:.5}  Z={:.5}",
                xyz[0], xyz[1], xyz[2]
            );
            println!(
                "  Rec.2020 spectral:R={:.5}  G={:.5}  B={:.5}",
                rec2020[0], rec2020[1], rec2020[2]
            );
            println!(
                "       Adobe RGB xy:R={:.5}  G={:.5}  B={:.5}",
                adobe_rgb[0], adobe_rgb[1], adobe_rgb[2]
            );
            println!(
                "            sRGB xy:R={:.5}  G={:.5}  B={:.5}",
                srgb_r, srgb_g, srgb_b
            );
            println!(" 8-bit");
            println!(
                "    VSF RGB gamma 2:R={:3}      G={:3}      B={:3}",
                vsf_r8, vsf_g8, vsf_b8
            );
            println!(
                "        sRGB hybrid:R={:3}      G={:3}      B={:3}",
                srgb_r8, srgb_g8, srgb_b8
            );
            println!(
                "    Rec.2020 hybrid:R={:3}      G={:3}      B={:3}",
                rec2020_r8, rec2020_g8, rec2020_b8
            );
            println!(
                "    Adobe RGB gamma:R={:3}      G={:3}      B={:3}",
                adobe_r8, adobe_g8, adobe_b8
            );
            println!(" 16-bit");
            println!(
                "        sRGB hybrid:R={:5}    G={:5}    B={:5}",
                srgb_r16, srgb_g16, srgb_b16
            );
            println!(
                "    Rec.2020 hybrid:R={:5}    G={:5}    B={:5}",
                rec2020_r16, rec2020_g16, rec2020_b16
            );
            println!(
                "    Adobe RGB gamma:R={:5}    G={:5}    B={:5}",
                adobe_r16, adobe_g16, adobe_b16
            );
            println!();
        }
    }

    println!("=======================================================================");
}

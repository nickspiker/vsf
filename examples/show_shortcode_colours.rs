// Prints out shortcode colours in VSF linear, LMS cone space, and XYZ

use vsf::colour::convert::apply_matrix_3x3;
use vsf::colour::spectral::constants::VSF_RGB2LMS;
use vsf::colour::legacy::constants::VSF_RGB2XYZ;
use vsf::types::vsf_type::VsfType;

fn main() {
    // Define shortcode colours using VsfType constants
    let colours = [
        ("Black",   "rk", VsfType::rk),
        ("White",   "rw", VsfType::rw),
        ("Grey",    "rg", VsfType::rg),
        ("Red",     "rr", VsfType::rr),
        ("Green",   "rn", VsfType::rn),
        ("Blue",    "rb", VsfType::rb),
        ("Cyan",    "rc", VsfType::rc),
        ("Magenta", "rj", VsfType::rj),
        ("Yellow",  "ry", VsfType::ry),
        ("Orange",  "ro", VsfType::ro),
        ("Lime",    "rl", VsfType::rl),
        ("Aqua",    "rq", VsfType::rq),
        ("Purple",  "rv", VsfType::rv),
    ];

    println!("\nVSF Shortcode Colours");
    println!("=======================================================================\n");

    for (name, code, vsf_type) in colours.iter() {
        if let Some(rgb_linear) = vsf_type.to_rgb_linear() {
            let rgb = [rgb_linear.r, rgb_linear.g, rgb_linear.b];
            let lms = apply_matrix_3x3(&VSF_RGB2LMS, &rgb);
            let lms_sum = lms[0] + lms[1] + lms[2];
            let xyz = apply_matrix_3x3(&VSF_RGB2XYZ, &rgb);

            println!("Colour: {} ({})", name, code);
            println!("  VSF RGB (linear):  R={:.3}  G={:.3}  B={:.3}", rgb[0], rgb[1], rgb[2]);
            println!("  LMS (cone space):  L={:.3}  M={:.3}  S={:.3}  (sum={:.3})", lms[0], lms[1], lms[2], lms_sum);
            println!("  XYZ (CIE 1931):    X={:.3}  Y={:.3}  Z={:.3}", xyz[0], xyz[1], xyz[2]);
            println!();
        }
    }

    println!("=======================================================================");
}

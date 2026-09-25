//! The audio grid (LOCK spec §1.2, §3): every sample Photon captures has a globally unique integer name.
//!
//! `SampleIndex` = samples since the Eagle epoch at a fixed 48 kHz, so sample `k` sits nominally at `k / 48000` seconds after the epoch, exact in rationals. A packet is 960 samples (20 ms) and packet boundaries fall on multiples of 960 counted from the top of each Eagle second — 50 packets a second, and because 48000 is a multiple of 960, the first sample of every Eagle second is a packet start.
//! Grid names are exact; an Eagle stamp is a MEASUREMENT of where a named sample actually was, and the difference is phase error. Integers name things; the one float here lives inside an estimator (`phase_error_samples`).

use crate::types::eagle_time::OSCILLATIONS_PER_SECOND;

/// Oscillations per Eagle second, widened for exact rational arithmetic.
pub const OPS: i128 = OSCILLATIONS_PER_SECOND as i128;
/// The one sample rate every device names samples at.
pub const RATE: i128 = 48_000;
/// Samples per packet (20 ms).
pub const PACKET: i64 = 960;

/// The Eagle count (nearest oscillation, half up) at which grid sample `k` nominally sits.
pub fn sample_to_eagle(k: i64) -> i64 {
    let n = k as i128 * OPS;
    (n.div_euclid(RATE) + ((n.rem_euclid(RATE) * 2 >= RATE) as i128)) as i64
}

/// The grid sample (nearest, half up) nominally at Eagle count `e`.
pub fn eagle_to_sample(e: i64) -> i64 {
    let n = e as i128 * RATE;
    (n.div_euclid(OPS) + ((n.rem_euclid(OPS) * 2 >= OPS) as i128)) as i64
}

/// Signed phase error in samples: the measured instant minus the nominal slot of `k`. An estimator's input, never a name.
pub fn phase_error_samples(k: i64, measured: i64) -> f64 {
    (measured - sample_to_eagle(k)) as f64 * RATE as f64 / OPS as f64
}

/// The first sample of the packet containing `k`.
pub fn packet_start(k: i64) -> i64 {
    k.div_euclid(PACKET) * PACKET
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> impl Iterator<Item = i64> {
        let mut x: u64 = 0xD1B5_4A32_D192_ED03;
        (0..200_000)
            .map(move |_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                // ±3e14 samples is ±198 years at 48 kHz — inside the i64 Eagle range (±206 years).
                (x % 600_000_000_000_000) as i64 - 300_000_000_000_000
            })
            .chain([-1, 0, 1, 959, 960, 47_999, 48_000, -48_000])
    }

    /// Naming is lossless: every sample survives the trip to its Eagle slot and back, exactly.
    #[test]
    fn sample_eagle_round_trip_is_exact() {
        for k in samples() {
            assert_eq!(eagle_to_sample(sample_to_eagle(k)), k, "k = {k}");
        }
    }

    /// The Eagle slot is the nearest oscillation to the exact rational k·OPS/RATE — never more than half an oscillation off.
    #[test]
    fn eagle_slot_within_half_an_oscillation() {
        for k in samples() {
            let exact_x2 = k as i128 * OPS * 2; // twice the rational numerator over RATE
            let got_x2 = sample_to_eagle(k) as i128 * RATE * 2;
            assert!((got_x2 - exact_x2).abs() <= RATE, "k = {k}");
        }
    }

    /// The top of every Eagle second is a packet start, and 50 packets tile a second.
    #[test]
    fn top_of_second_is_a_packet_start() {
        for sec in [-3i64, 0, 1, 1_000_000_000] {
            let k = sec * RATE as i64;
            assert_eq!(packet_start(k), k);
            assert_eq!(sample_to_eagle(k), sec * OPS as i64, "a whole second lands on a whole-second count");
        }
        assert_eq!(RATE as i64 / PACKET, 50);
        assert_eq!(packet_start(961), 960);
        assert_eq!(packet_start(-1), -960);
    }

    /// Phase error reads zero on the nominal slot and one sample per sample's worth of oscillations.
    #[test]
    fn phase_error_is_in_samples() {
        let k = 123_456;
        assert!(phase_error_samples(k, sample_to_eagle(k)).abs() < 1e-4);
        assert!((phase_error_samples(k, sample_to_eagle(k + 1)) - 1.0).abs() < 1e-4);
    }
}

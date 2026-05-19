//! Official VSF section schemas
//!
//! These schemas define the standard fields for common VSF section types. They are automatically registered in SchemaRegistry::global()

use super::constraint::TypeConstraint;
use super::section::SectionSchema;

/// Create image metadata schema
/// Common fields for image data sections
pub fn image_schema() -> SectionSchema {
    SectionSchema::new("image")
        .description("Image capture metadata")
        .field("width", TypeConstraint::AnyUnsigned)
        .field("height", TypeConstraint::AnyUnsigned)
        .field("format", TypeConstraint::AnyString) // e.g., "RGB", "RGBA", "YUV420"
        .field("bit_depth", TypeConstraint::AnyUnsigned) // bits per channel
        .field("timestamp", TypeConstraint::AnyEagleTime)
        .field("exposure_time", TypeConstraint::AnyFloat) // seconds
        .field("iso", TypeConstraint::AnyUnsigned)
        .field("focal_length", TypeConstraint::AnyFloat) // mm
        .field("aperture", TypeConstraint::AnyFloat) // f-number
}

/// Create camera configuration schema
/// Settings and calibration for camera hardware
pub fn camera_schema() -> SectionSchema {
    SectionSchema::new("camera")
        .description("Camera hardware configuration")
        .field("model", TypeConstraint::AnyString)
        .field("serial", TypeConstraint::AnyString)
        .field("sensor_width", TypeConstraint::AnyFloat) // mm
        .field("sensor_height", TypeConstraint::AnyFloat) // mm
        .field("resolution_x", TypeConstraint::AnyUnsigned)
        .field("resolution_y", TypeConstraint::AnyUnsigned)
        .field("pixel_size", TypeConstraint::AnyFloat) // micrometers
        .field("calibrated", TypeConstraint::AnyUnsigned) // bool: 0 or 1
        .field("timestamp", TypeConstraint::AnyEagleTime)
}

/// Create audio stream schema
/// Metadata for audio data
pub fn audio_schema() -> SectionSchema {
    SectionSchema::new("audio")
        .description("Audio stream metadata")
        .field("sample_rate", TypeConstraint::AnyUnsigned) // Hz
        .field("channels", TypeConstraint::AnyUnsigned) // 1=mono, 2=stereo, etc.
        .field("bit_depth", TypeConstraint::AnyUnsigned) // bits per sample
        .field("format", TypeConstraint::AnyString) // e.g., "PCM", "FLAC", "Opus"
        .field("duration", TypeConstraint::AnyFloat) // seconds
        .field("timestamp", TypeConstraint::AnyEagleTime)
}

/// Create network peer schema
/// Information about network peers for P2P protocols
pub fn network_peer_schema() -> SectionSchema {
    SectionSchema::new("network_peer")
        .description("Network peer information")
        .field("handle_hash", TypeConstraint::Blake3Provenance)
        .field("device_pubkey", TypeConstraint::X25519Key)
        .field("ip_address", TypeConstraint::AnyString)
        .field("port", TypeConstraint::AnyUnsigned)
        .field("last_seen", TypeConstraint::AnyEagleTime)
        .field("protocol_version", TypeConstraint::AnyUnsigned)
}

/// Create announce schema for FGTW bootstrap
/// Used in challenge-response protocol
pub fn announce_schema() -> SectionSchema {
    SectionSchema::new("announce")
        .description("FGTW bootstrap announce message")
        .field("challenge_hash", TypeConstraint::Blake3Rolling)
        .field("handle_hash", TypeConstraint::Blake3Provenance)
        .field("port", TypeConstraint::AnyUnsigned)
        .field("protocol_version", TypeConstraint::AnyUnsigned)
}

// ============================================================================
// PIPE bridge protocol — single-cable USB-CDC commands and responses
// ============================================================================

/// `cmd` section: request from host CLI to bridge firmware (RP2040).
///
/// Top-level VSF document carries `(d"cmd": …)` with header `hp` (BLAKE3 provenance) covering the whole packet.
/// `op` is the verb (`ping`, `info`, `uptime`, `random`, `bootsel`, `reset`).
/// `id` is a per-cmd identifier echoed in the response so the host can correlate requests in flight.
/// `n` is an optional numeric argument (e.g., byte count for `random`).
pub fn bridge_cmd_schema() -> SectionSchema {
    SectionSchema::new("cmd")
        .field("op", TypeConstraint::AsciiText)
        .field("id", TypeConstraint::AnyUnsigned)
        .field("n", TypeConstraint::AnyUnsigned)
}

/// `resp` section: response from bridge firmware to host CLI.
///
/// Only the fields relevant to the matched `op` are populated; the schema is intentionally permissive so a single section type covers ping/info/uptime/random/error responses.
/// `op` carries the response verb on success (e.g., `pong`); `err` carries an error code string instead when `op` would fail.
/// `did` is the bridge's BLAKE3-derived device id (32 bytes) — populated only by `info`.
/// `data` carries the random bytes payload for `random` as a wrapped binary blob (`v(b'b', bytes)`).
pub fn bridge_resp_schema() -> SectionSchema {
    SectionSchema::new("resp")
        .field("id", TypeConstraint::AnyUnsigned)
        .field("op", TypeConstraint::AsciiText)
        .field("err", TypeConstraint::AsciiText)
        .field("name", TypeConstraint::AsciiText)
        .field("ver", TypeConstraint::AnyUnsigned)
        .field("did", TypeConstraint::Blake3Provenance)
        .field("clk", TypeConstraint::AnyUnsigned)
        .field("up", TypeConstraint::AnyUnsigned)
        .field("ms", TypeConstraint::AnyUnsigned)
        .field("data", TypeConstraint::Wrapped(b'b'))
}

/// Register all official schemas.
/// Gated on `registry` because `SchemaRegistry` itself is — see `schema/mod.rs`.
#[cfg(feature = "registry")]
pub fn register_official_schemas(registry: &super::registry::SchemaRegistry) {
    registry.register(image_schema());
    registry.register(camera_schema());
    registry.register(audio_schema());
    registry.register(network_peer_schema());
    registry.register(announce_schema());
    registry.register(bridge_cmd_schema());
    registry.register(bridge_resp_schema());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::SchemaRegistry;

    #[test]
    fn test_image_schema_creation() {
        let schema = image_schema();
        assert_eq!(schema.name, "image");
        assert!(schema.fields.len() >= 5);
    }

    #[test]
    fn test_camera_schema_creation() {
        let schema = camera_schema();
        assert_eq!(schema.name, "camera");
        assert!(schema.fields.len() >= 5);
    }

    #[test]
    fn test_audio_schema_creation() {
        let schema = audio_schema();
        assert_eq!(schema.name, "audio");
        assert!(schema.fields.iter().any(|f| f.name == "sample_rate"));
    }

    #[test]
    fn test_network_peer_schema() {
        let schema = network_peer_schema();
        assert_eq!(schema.name, "network_peer");
        assert!(schema.fields.iter().any(|f| f.name == "handle_hash"));
    }

    #[test]
    fn test_announce_schema() {
        let schema = announce_schema();
        assert_eq!(schema.name, "announce");
        assert!(schema.fields.iter().any(|f| f.name == "challenge_hash"));
    }

    #[test]
    fn test_register_all() {
        let registry = SchemaRegistry::new();
        register_official_schemas(&registry);

        assert!(registry.get("image").is_ok());
        assert!(registry.get("camera").is_ok());
        assert!(registry.get("audio").is_ok());
        assert!(registry.get("network_peer").is_ok());
        assert!(registry.get("announce").is_ok());
    }
}

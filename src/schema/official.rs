//! Official VSF section schemas
//!
//! These schemas define the standard fields for common VSF section types. They are automatically registered in SchemaRegistry::global()

use super::constraint::TypeConstraint;
use super::section::SectionSchema;

/// Create image metadata schema Common fields for image data sections
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

/// Create camera configuration schema Settings and calibration for camera hardware
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

/// Create audio stream schema Metadata for audio data
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

/// Create network peer schema Information about network peers for P2P protocols
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

/// Create announce schema for FGTW bootstrap Used in challenge-response protocol
pub fn announce_schema() -> SectionSchema {
    SectionSchema::new("announce")
        .description("FGTW bootstrap announce message")
        .field("challenge_hash", TypeConstraint::Blake3Rolling)
        .field("handle_hash", TypeConstraint::Blake3Provenance)
        .field("port", TypeConstraint::AnyUnsigned)
        .field("protocol_version", TypeConstraint::AnyUnsigned)
}

// ============================================================================
// PIPE protocol — host CLI ↔ PIPE bridge (and eventually PIPE silicon) over USB-CDC
// ============================================================================

/// `pipe_message` — every wire packet on a PIPE link is a complete VSF document with this single section.
///
/// One registry entry covers both directions of the protocol — USB transfer direction (IN vs OUT) already tells you who sent it, and the `op` field carries the verb (`ping`/`pong`/`info`/`uptime`/`random`/`bootsel`/`reset`/`err`). Section body is the standard small-section shape `[(field)(field)…]`; section name lives in the TOC entry only (bridge packets are <200 B so we are well under the 1 MB threshold that would require the body to repeat name+count+length).
///
/// Field roles by direction:
///
/// * host → device (cmd): `op` ∈ {`ping`, `info`, `uptime`, `random`, `bootsel`, `reset`}, `id` (correlation id echoed back), optionally `n` (numeric arg, e.g. byte count for `random`).
/// * device → host (resp): always `id` (echoes cmd's id). On success: `op` = `pong` for ping; info-fields {`name`, `ver`, `did`, `clk`, `up`} for info; `ms` for uptime; `data` for random. On unknown op: `err` carries the error code string.
///
/// All fields are optional; the schema accepts any subset and protocol semantics are field-driven. Vendors define what fields make sense for their device inside the standard "pipe_message" envelope.
pub fn pipe_message_schema() -> SectionSchema {
    SectionSchema::new("pipe_message")
        .field("op", TypeConstraint::AsciiText)
        .field("id", TypeConstraint::AnyUnsigned)
        .field("n", TypeConstraint::AnyUnsigned)
        .field("err", TypeConstraint::AsciiText)
        .field("name", TypeConstraint::AsciiText)
        .field("ver", TypeConstraint::AnyUnsigned)
        .field("did", TypeConstraint::Blake3Provenance)
        .field("clk", TypeConstraint::AnyUnsigned)
        .field("up", TypeConstraint::AnyUnsigned)
        .field("ms", TypeConstraint::AnyUnsigned)
        .field("data", TypeConstraint::Wrapped(b'b'))
        // pipe-bridge: PIPE comms state from the most recent fast_sync/wire_mirror. 0 = unlocked / never synced, 1 = lock held (256+ consecutive in-threshold edges per polarity, well within the proven 2^12-slot stability margin).
        .field("pipe", TypeConstraint::AnyUnsigned)
}

/// Create error schema — the standard shape for a worker/protocol error frame.
///
/// `reason` is the short machine-readable cause; `detail` is an optional human-readable elaboration.
/// Both are free-form strings so any subsystem can populate the envelope without a bespoke schema.
pub fn error_schema() -> SectionSchema {
    SectionSchema::new("error")
        .field("reason", TypeConstraint::AnyString)
        .field("detail", TypeConstraint::AnyString)
}

/// Register all official schemas. Gated on `registry` because `SchemaRegistry` itself is — see `schema/mod.rs`.
#[cfg(feature = "registry")]
pub fn register_official_schemas(registry: &super::registry::SchemaRegistry) {
    registry.register(image_schema());
    registry.register(camera_schema());
    registry.register(audio_schema());
    registry.register(network_peer_schema());
    registry.register(announce_schema());
    registry.register(pipe_message_schema());
    registry.register(error_schema());
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

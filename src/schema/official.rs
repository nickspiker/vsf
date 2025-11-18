//! Official VSF section schemas
//!
//! These schemas define the standard fields for common VSF section types.
//! They are automatically registered in SchemaRegistry::global()

use super::field::FieldType;
use super::section::SectionSchema;

/// Create image metadata schema
/// Common fields for image data sections
pub fn image_schema() -> SectionSchema {
    SectionSchema::new("image")
        .description("Image capture metadata")
        .field("width", FieldType::U32)
        .field("height", FieldType::U32)
        .field("format", FieldType::String) // e.g., "RGB", "RGBA", "YUV420"
        .field("bit_depth", FieldType::U8) // bits per channel
        .field("timestamp", FieldType::EagleTimeF64)
        .field("exposure_time", FieldType::F64) // seconds
        .field("iso", FieldType::U16)
        .field("focal_length", FieldType::F32) // mm
        .field("aperture", FieldType::F32) // f-number
}

/// Create camera configuration schema
/// Settings and calibration for camera hardware
pub fn camera_schema() -> SectionSchema {
    SectionSchema::new("camera")
        .description("Camera hardware configuration")
        .field("model", FieldType::String)
        .field("serial", FieldType::String)
        .field("sensor_width", FieldType::F32) // mm
        .field("sensor_height", FieldType::F32) // mm
        .field("resolution_x", FieldType::U32)
        .field("resolution_y", FieldType::U32)
        .field("pixel_size", FieldType::F32) // micrometers
        .field("calibrated", FieldType::U8) // bool: 0 or 1
        .field("timestamp", FieldType::EagleTimeF64)
}

/// Create audio stream schema
/// Metadata for audio data
pub fn audio_schema() -> SectionSchema {
    SectionSchema::new("audio")
        .description("Audio stream metadata")
        .field("sample_rate", FieldType::U32) // Hz
        .field("channels", FieldType::U8) // 1=mono, 2=stereo, etc.
        .field("bit_depth", FieldType::U8) // bits per sample
        .field("format", FieldType::String) // e.g., "PCM", "FLAC", "Opus"
        .field("duration", FieldType::F64) // seconds
        .field("timestamp", FieldType::EagleTimeF64)
}

/// Create network peer schema
/// Information about network peers for P2P protocols
pub fn network_peer_schema() -> SectionSchema {
    SectionSchema::new("network_peer")
        .description("Network peer information")
        .field("handle_hash", FieldType::Blake3Hash)
        .field("device_pubkey", FieldType::X25519Key)
        .field("ip_address", FieldType::String)
        .field("port", FieldType::U16)
        .field("last_seen", FieldType::EagleTimeF64)
        .field("protocol_version", FieldType::U16)
}

/// Create announce schema for FGTW bootstrap
/// Used in challenge-response protocol
pub fn announce_schema() -> SectionSchema {
    SectionSchema::new("announce")
        .description("FGTW bootstrap announce message")
        .field("challenge_hash", FieldType::Blake3Hash)
        .field("handle_hash", FieldType::Blake3Hash)
        .field("port", FieldType::U16)
        .field("protocol_version", FieldType::U16)
}

/// Register all official schemas
pub fn register_official_schemas(registry: &super::registry::SchemaRegistry) {
    registry.register(image_schema());
    registry.register(camera_schema());
    registry.register(audio_schema());
    registry.register(network_peer_schema());
    registry.register(announce_schema());
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
        assert_eq!(schema.fields.iter().find(|f| f.name == "sample_rate").unwrap().field_type, FieldType::U32);
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

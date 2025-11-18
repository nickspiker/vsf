# VSF Schema System

Type-safe validation for VSF sections using pattern-based constraints that work directly with VsfType primitives.

## Core Concept

Instead of duplicating VsfType's 211 variants into parallel type enums, the schema system uses **TypeConstraint** - a pattern-based validation approach that matches against VsfType variants directly.

```rust
use vsf::schema::{SectionSchema, TypeConstraint, IntoVsfType, FromVsfType};
use vsf::VsfType;

// Define a schema using TypeConstraint
let schema = SectionSchema::new("camera")
    .field("iso", TypeConstraint::AnyUnsigned)
    .field("aperture", TypeConstraint::AnyFloat)
    .field("timestamp", TypeConstraint::AnyEagleTime);

// Build a section with automatic type conversion
let section = schema.builder()
    .set("iso", 800u32)?                    // u32 → VsfType::u5(800)
    .set("aperture", 2.8f32)?               // f32 → VsfType::f5(2.8)
    .set("timestamp", VsfType::e(EtType::f6(1234567890.0)))?
    .build()?;
```

## TypeConstraint Patterns

TypeConstraint provides category-based validation without duplicating types:

```rust
// Numeric categories
TypeConstraint::AnyUnsigned    // Matches u0, u3-u7, u(...)
TypeConstraint::AnySigned      // Matches i3-i7, i(...)
TypeConstraint::AnyInteger     // Any unsigned or signed
TypeConstraint::AnyFloat       // Matches f5, f6
TypeConstraint::AnyNumeric     // Any integer or float
TypeConstraint::AnyComplex     // Matches j5, j6, Spirix Circle types

// Eagle Time variants
TypeConstraint::AnyEagleTime   // Any e(EtType::*)
TypeConstraint::EagleTime(Box::new(TypeConstraint::AnyFloat))  // e(f5) or e(f6) only

// Hash types (all are hp, hb, or hs variants)
TypeConstraint::AnyHash        // Any hash type
TypeConstraint::Blake3Provenance  // hp specifically
TypeConstraint::Blake3Rolling     // hb specifically

// Cryptographic keys
TypeConstraint::AnyKey         // Any key type (ke, kx, kp, kc, ka)
TypeConstraint::Ed25519Key     // ke specifically (32 bytes)
TypeConstraint::X25519Key      // kx specifically (32 bytes)

// Strings
TypeConstraint::AnyString      // Matches x, d, l

// Composite constraints
TypeConstraint::OneOf(vec![
    TypeConstraint::AnyUnsigned,
    TypeConstraint::AnyFloat
])  // Accept unsigned OR float

TypeConstraint::AllOf(vec![
    TypeConstraint::AnyNumeric,
    TypeConstraint::Custom { ... }
])  // Must satisfy both

// Custom validators
TypeConstraint::Custom {
    name: "positive",
    description: "Must be positive",
    validator: Arc::new(|vsf: &VsfType| {
        match vsf {
            VsfType::u5(v) => *v > 0,
            VsfType::f6(v) => *v > 0.0,
            _ => false
        }
    })
}
```

## Complete Examples

### Example 1: Network Peer Schema

```rust
use vsf::schema::{SectionSchema, TypeConstraint, SectionBuilder};
use vsf::VsfType;

// Define schema for network peer data
let peer_schema = SectionSchema::new("network_peer")
    .description("P2P network peer information")
    .field("handle_hash", TypeConstraint::Blake3Provenance)
    .field("device_pubkey", TypeConstraint::X25519Key)
    .field("ip_address", TypeConstraint::AnyString)
    .field("port", TypeConstraint::AnyUnsigned)
    .field("last_seen", TypeConstraint::AnyEagleTime)
    .field("protocol_version", TypeConstraint::AnyUnsigned);

// Build a peer record with automatic validation
let peer = peer_schema.builder()
    .set("handle_hash", VsfType::hp(hash_bytes))?       // Must be hp
    .set("device_pubkey", VsfType::kx(pubkey_bytes))?   // Must be kx
    .set("ip_address", "192.168.1.100".to_string())?    // String → VsfType::x
    .set("port", 41641u16)?                             // u16 → VsfType::u4
    .set("last_seen", VsfType::e(EtType::f6(timestamp)))?
    .set("protocol_version", 1u32)?                     // u32 → VsfType::u5
    .build()?;

// Extract values with type safety
let port: u16 = peer.get_value("port")?;                // Automatic extraction
let ip: String = peer.get_value("ip_address")?;
```

### Example 2: Image Metadata Schema

```rust
use vsf::schema::{SectionSchema, TypeConstraint};

let image_schema = SectionSchema::new("image")
    .description("Camera image metadata")
    .field("width", TypeConstraint::AnyUnsigned)
    .field("height", TypeConstraint::AnyUnsigned)
    .field("format", TypeConstraint::AnyString)
    .field("bit_depth", TypeConstraint::AnyUnsigned)
    .field("timestamp", TypeConstraint::AnyEagleTime)
    .field("exposure_time", TypeConstraint::AnyFloat)
    .field("iso", TypeConstraint::AnyUnsigned)
    .field("focal_length", TypeConstraint::AnyFloat)
    .field("aperture", TypeConstraint::AnyFloat);

// Build with validation
let metadata = image_schema.builder()
    .set("width", 4096u32)?
    .set("height", 3072u32)?
    .set("format", "RGGB".to_string())?
    .set("bit_depth", 12u8)?
    .set("timestamp", VsfType::e(EtType::f6(time)))?
    .set("exposure_time", 0.01f64)?     // 1/100s
    .set("iso", 800u32)?
    .set("focal_length", 0.024f32)?     // 24mm in meters
    .set("aperture", 2.8f32)?           // f/2.8
    .build()?;
```

### Example 3: Required vs Optional Fields

```rust
let schema = SectionSchema::new("user")
    .field_required("user_id", TypeConstraint::AnyUnsigned)    // Must be present
    .field_required("username", TypeConstraint::AnyString)     // Must be present
    .field("email", TypeConstraint::AnyString)                 // Optional
    .field_with_default("role", "user".into_vsf_type());       // Optional with default

// Missing required field = validation error
let result = schema.builder()
    .set("username", "alice".to_string())?
    // ERROR: user_id is required but missing
    .build();
assert!(result.is_err());

// With required fields present
let user = schema.builder()
    .set("user_id", 42u32)?
    .set("username", "alice".to_string())?
    // email and role are optional
    .build()?;
```

### Example 4: FGTW Announce Message

```rust
// Official schema from vsf/src/schema/official.rs
let announce_schema = SectionSchema::new("announce")
    .description("FGTW bootstrap announce message")
    .field("challenge_hash", TypeConstraint::Blake3Rolling)
    .field("handle_hash", TypeConstraint::Blake3Provenance)
    .field("port", TypeConstraint::AnyUnsigned)
    .field("protocol_version", TypeConstraint::AnyUnsigned);

// Build announce message
let announce = announce_schema.builder()
    .set("challenge_hash", VsfType::hb(challenge))?    // Must be hb
    .set("handle_hash", VsfType::hp(handle))?          // Must be hp
    .set("port", 41641u16)?
    .set("protocol_version", 1u32)?
    .build()?;

// Validate existing data
announce_schema.validate(&section)?;  // Returns Ok(()) or ValidationError
```

## Type Conversions

The `IntoVsfType` and `FromVsfType` traits provide automatic conversions:

### IntoVsfType (Rust → VsfType)

```rust
use vsf::schema::IntoVsfType;

// Primitives
let u: VsfType = 42u32.into_vsf_type();          // VsfType::u5(42)
let i: VsfType = (-100i32).into_vsf_type();      // VsfType::i5(-100)
let f: VsfType = 3.14f64.into_vsf_type();        // VsfType::f6(3.14)
let b: VsfType = true.into_vsf_type();           // VsfType::u0(true)

// Strings
let s: VsfType = "hello".into_vsf_type();        // VsfType::x("hello")
let s2: VsfType = "world".to_string().into_vsf_type();

// Bytes
let bytes: VsfType = vec![0u8; 32].into_vsf_type();  // VsfType::hb(vec![...])

// VsfType passthrough
let vsf = VsfType::u5(100);
let same: VsfType = vsf.into_vsf_type();         // No conversion needed
```

### FromVsfType (VsfType → Rust)

```rust
use vsf::schema::FromVsfType;

let vsf = VsfType::u5(42);
let n: u32 = u32::from_vsf_type(&vsf)?;          // Ok(42)
let n2: u64 = u64::from_vsf_type(&vsf)?;         // Ok(42u64) - cross-size works

// Cross-size conversions (with bounds checking)
let small = VsfType::u3(100u8);
let big: u32 = u32::from_vsf_type(&small)?;      // Ok(100u32)

// Type mismatch = error
let vsf = VsfType::f6(3.14);
let result = u32::from_vsf_type(&vsf);
assert!(result.is_err());  // Cannot convert f6 to u32

// Strings
let vsf = VsfType::x("test".to_string());
let s: String = String::from_vsf_type(&vsf)?;    // Ok("test")

// Also accepts VsfType::d and VsfType::l for strings
```

## Schema Registry

Register schemas globally for reuse across your application:

```rust
use vsf::schema::{SchemaRegistry, register_official_schemas};

// Get global registry
let registry = SchemaRegistry::global();

// Official schemas are pre-registered
register_official_schemas(&registry);

// Use registered schemas
let image_schema = registry.get("image")?;
let camera_schema = registry.get("camera")?;
let audio_schema = registry.get("audio")?;

// Register custom schemas
registry.register(
    SectionSchema::new("my_custom")
        .field("data", TypeConstraint::AnyNumeric)
);

let custom = registry.get("my_custom")?;
```

## Validation Errors

The schema system provides detailed error messages:

```rust
use vsf::schema::ValidationError;

// Unknown field
ValidationError::UnknownField {
    section: "image".to_string(),
    field: "invalid".to_string(),
    allowed: vec!["width", "height", "format"]
}
// "Field 'invalid' not valid for section 'image'. Allowed fields: width, height, format"

// Type mismatch
ValidationError::TypeConstraintViolation {
    constraint: "AnyUnsigned".to_string(),
    got: "f6".to_string()
}
// "Type constraint violation: expected AnyUnsigned, got f6"

// Missing required field
ValidationError::MissingField {
    section: "user".to_string(),
    field: "user_id".to_string()
}
// "Required field 'user_id' missing in section 'user'"
```

## Official Schemas

VSF provides pre-built schemas for common use cases in [official.rs](official.rs):

- **image** - Image capture metadata (width, height, format, timestamp, exposure, ISO, etc.)
- **camera** - Camera hardware configuration (model, serial, sensor specs, calibration)
- **audio** - Audio stream metadata (sample rate, channels, bit depth, format, duration)
- **network_peer** - P2P peer information (handle hash, pubkey, IP, port, last seen)
- **announce** - FGTW bootstrap announce message (challenge hash, handle, port, version)

All official schemas are automatically registered in `SchemaRegistry::global()`.

## Architecture Benefits

This approach provides:

**Complete type coverage** - All 211 VsfType variants supported without maintaining parallel enums

**No duplication** - TypeConstraint patterns match against VsfType directly, not duplicates

**Compile-time safety** - Rust's type system ensures exhaustive matching in constraint validation

**Extensible** - Add custom validators via TypeConstraint::Custom without modifying core code

**Ergonomic** - Automatic type conversion via IntoVsfType/FromVsfType traits

**Future-proof** - New VsfType variants automatically supported by category constraints (AnyUnsigned, etc.)

## Migration from Raw VsfType

Before schemas:
```rust
// Manual construction, no validation
let mut fields = HashMap::new();
fields.insert("iso".to_string(), VsfType::u5(800));
fields.insert("aperture".to_string(), VsfType::f5(2.8));
// Easy to make mistakes, no type checking
```

With schemas:
```rust
// Type-safe, validated, automatic conversion
let section = camera_schema.builder()
    .set("iso", 800u32)?          // Validated as AnyUnsigned
    .set("aperture", 2.8f32)?     // Validated as AnyFloat
    .build()?;                    // All required fields checked
```

## See Also

- [constraint.rs](constraint.rs) - TypeConstraint implementation
- [conversions.rs](conversions.rs) - IntoVsfType/FromVsfType trait implementations
- [section.rs](section.rs) - SectionSchema and SectionBuilder
- [official.rs](official.rs) - Pre-built schemas for common use cases
- [validate.rs](validate.rs) - Validation error types

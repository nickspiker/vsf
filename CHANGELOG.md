# Changelog

All notable changes to VSF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-05

### Added
- **Opcode type (`VsfType::op`)** - Added dedicated type for executable bytecode with two-character ASCII identifiers
- **Literal VSF format** - `vsfinfo` now displays 1:1 file representation showing exact wire format
- **Proper bracket notation** - Semantic distinction between `⦉⦊` (interpreted values) and `{}` (opcodes)
- **G^0* base notation** - Hexadecimal display with proper mathematical base prefix (replaces legacy 0x)

### Fixed
- **Eagle Time millisecond precision** - Now correctly calculates milliseconds from oscillation counts for integer types (eu6, ei6) instead of truncating to .000
- **Removed DEBUG prints** - Cleaned up debug output from Eagle Time metadata parsing

### Changed
- **BREAKING**: VSF format version bumped to 7 (z7 y7)
- **BREAKING**: Backward compatibility set to v7 (cannot read v6 files due to opcode type addition)
- **Type size markers** - All type size indicators now use `⦉⦊` brackets (e.g., `z3⦉6⦊`, `hp3⦉31⦊`)
- **Crypto hash formatting** - Hash type sizes now use `⦉⦊` brackets consistently with other types

### Documentation
- Added link to full documentation at https://holdmyoscilloscope.com/vsf/
- Updated README with v0.3.0 feature descriptions
- Moved capability tokens to v0.4.0 roadmap

## [0.2.3] - 2025-01-XX

### Previous releases
- Camera RAW support with bit-packed tensors
- Ed25519 signatures and verification
- Eagle Time temporal encoding
- Huffman text compression
- Spirix arithmetic integration

---

[0.3.0]: https://github.com/nickspiker/vsf/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/nickspiker/vsf/releases/tag/v0.2.3

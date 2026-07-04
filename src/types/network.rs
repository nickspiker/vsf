//! Network and physical address types for VSF.
//!
//! # Network family (n + lowercase)
//! - `ni`: IPv4 address (4 bytes)
//! - `nj`: IPv6 address (16 bytes)
//! - `nh`: Bare hostname or IP string
//! - `na`: Full address with scheme + host + optional port
//! - `nc`: CIDR block (address string + prefix length)
//! - `nm`: MAC address (6 bytes)
//! - `np`: Standalone port number
//! - `ns`: Socket (host:port, no scheme)
//! - `nu`: Full URL including path/query
//! - `nn`: DNS name (validated format)
//!
//! # World address family (w + lowercase)
//! - `ww`: World coordinate (Dymaxion icosahedral, a raw u64)
//! - `wa`: Postal address (structured, all fields optional)

use crate::decoding::traits::DecodeError;
use crate::prelude::*;
#[cfg(feature = "std")]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use super::vsf_type::VsfType;

// ============================================================
// NaScheme — typed scheme byte for VsfType::na ============================================================

/// Scheme identifier for `VsfType::na` (network address).
///
/// The scheme occupies one byte on the wire.  Values 0–8 are assigned by the VSF spec; values 9–255 are reserved.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaScheme {
    Https = 0,
    Http = 1,
    Ftp = 2,
    Sftp = 3,
    Ssh = 4,
    Ntp = 5,
    Smtp = 6,
    Ws = 7,
    Wss = 8,
}

impl NaScheme {
    /// Decode from the wire byte.  Returns `None` for unknown values.
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::Https,
            1 => Self::Http,
            2 => Self::Ftp,
            3 => Self::Sftp,
            4 => Self::Ssh,
            5 => Self::Ntp,
            6 => Self::Smtp,
            7 => Self::Ws,
            8 => Self::Wss,
            _ => return None,
        })
    }

    pub const fn as_byte(self) -> u8 {
        self as u8
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Http => "http",
            Self::Ftp => "ftp",
            Self::Sftp => "sftp",
            Self::Ssh => "ssh",
            Self::Ntp => "ntp",
            Self::Smtp => "smtp",
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }

    /// Well-known default port for this scheme, if any.
    pub const fn default_port(self) -> Option<u16> {
        Some(match self {
            Self::Https => 443,
            Self::Http => 80,
            Self::Ftp => 21,
            Self::Sftp => 22,
            Self::Ssh => 22,
            Self::Ntp => 123,
            Self::Smtp => 25,
            Self::Ws => 80,
            Self::Wss => 443,
        })
    }
}

impl core::fmt::Display for NaScheme {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================
// WaAddress — structured postal address ============================================================

/// Structured postal address for `VsfType::wa`.
///
/// All fields are optional.  Fields present on the wire are indicated by a one-byte presence bitmask followed by the data in order:
///
/// ```text
/// [mask: u8] bit 0 — line1       (null-terminated UTF-8) bit 1 — line2       (null-terminated UTF-8) bit 2 — city        (null-terminated UTF-8) bit 3 — region      (null-terminated UTF-8, state/province/territory) bit 4 — postal_code (null-terminated UTF-8) bit 5 — country     (2 bytes, ISO 3166-1 alpha-2, e.g. b"US") bits 6-7 — reserved, must be zero [line1 if bit 0] [line2 if bit 1] [city if bit 2] [region if bit 3] [postal_code if bit 4] [country if bit 5]
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WaAddress {
    pub line1: Option<String>, // Street address
    pub line2: Option<String>, // Apt, suite, floor, etc.
    pub city: Option<String>,
    pub region: Option<String>,      // State, province, territory
    pub postal_code: Option<String>, // ZIP / postcode
    pub country: Option<[u8; 2]>,    // ISO 3166-1 alpha-2, e.g. *b"US"
}

impl WaAddress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_line1(mut self, v: impl Into<String>) -> Self {
        self.line1 = Some(v.into());
        self
    }
    pub fn with_line2(mut self, v: impl Into<String>) -> Self {
        self.line2 = Some(v.into());
        self
    }
    pub fn with_city(mut self, v: impl Into<String>) -> Self {
        self.city = Some(v.into());
        self
    }
    pub fn with_region(mut self, v: impl Into<String>) -> Self {
        self.region = Some(v.into());
        self
    }
    pub fn with_postal_code(mut self, v: impl Into<String>) -> Self {
        self.postal_code = Some(v.into());
        self
    }
    pub fn with_country(mut self, v: [u8; 2]) -> Self {
        self.country = Some(v);
        self
    }

    /// Encode to wire bytes (presence mask + fields). Does NOT include the `wa` tag bytes — those are added by the flattener.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut mask: u8 = 0;
        if self.line1.is_some() {
            mask |= 0x01;
        }
        if self.line2.is_some() {
            mask |= 0x02;
        }
        if self.city.is_some() {
            mask |= 0x04;
        }
        if self.region.is_some() {
            mask |= 0x08;
        }
        if self.postal_code.is_some() {
            mask |= 0x10;
        }
        if self.country.is_some() {
            mask |= 0x20;
        }

        let mut out = vec![mask];
        let push_str = |out: &mut Vec<u8>, s: &str| {
            out.extend_from_slice(s.as_bytes());
            out.push(0);
        };
        if let Some(s) = &self.line1 {
            push_str(&mut out, s);
        }
        if let Some(s) = &self.line2 {
            push_str(&mut out, s);
        }
        if let Some(s) = &self.city {
            push_str(&mut out, s);
        }
        if let Some(s) = &self.region {
            push_str(&mut out, s);
        }
        if let Some(s) = &self.postal_code {
            push_str(&mut out, s);
        }
        if let Some(c) = &self.country {
            out.extend_from_slice(c);
        }
        out
    }

    /// Decode from wire bytes (starting after the `wa` tag).
    pub fn from_wire(data: &[u8], pointer: &mut usize) -> Result<Self, DecodeError> {
        if *pointer >= data.len() {
            return Err(DecodeError::UnexpectedEofMsg("wa: missing mask".into()));
        }
        let mask = data[*pointer];
        *pointer += 1;

        let read_str = |data: &[u8], pointer: &mut usize| -> Result<String, DecodeError> {
            let start = *pointer;
            while *pointer < data.len() && data[*pointer] != 0 {
                *pointer += 1;
            }
            if *pointer >= data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "wa: unterminated string".into(),
                ));
            }
            let s = core::str::from_utf8(&data[start..*pointer])
                .map_err(|e| DecodeError::InvalidDataMsg(format!("{}", e)))?
                .to_string();
            *pointer += 1; // consume null
            Ok(s)
        };

        let mut addr = WaAddress::default();
        if mask & 0x01 != 0 {
            addr.line1 = Some(read_str(data, pointer)?);
        }
        if mask & 0x02 != 0 {
            addr.line2 = Some(read_str(data, pointer)?);
        }
        if mask & 0x04 != 0 {
            addr.city = Some(read_str(data, pointer)?);
        }
        if mask & 0x08 != 0 {
            addr.region = Some(read_str(data, pointer)?);
        }
        if mask & 0x10 != 0 {
            addr.postal_code = Some(read_str(data, pointer)?);
        }
        if mask & 0x20 != 0 {
            if *pointer + 2 > data.len() {
                return Err(DecodeError::UnexpectedEofMsg(
                    "wa: country truncated".into(),
                ));
            }
            addr.country = Some([data[*pointer], data[*pointer + 1]]);
            *pointer += 2;
        }
        Ok(addr)
    }

    /// Wire byte size (not including the `wa` tag bytes).
    pub fn wire_len(&self) -> usize {
        let str_len = |s: &Option<String>| s.as_ref().map_or(0, |v| v.len() + 1);
        1 // mask
        + str_len(&self.line1)
        + str_len(&self.line2)
        + str_len(&self.city)
        + str_len(&self.region)
        + str_len(&self.postal_code)
        + self.country.map_or(0, |_| 2)
    }
}

impl core::fmt::Display for WaAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut parts = Vec::new();
        if let Some(s) = &self.line1 {
            parts.push(s.as_str().to_string());
        }
        if let Some(s) = &self.line2 {
            parts.push(s.as_str().to_string());
        }
        if let Some(s) = &self.city {
            parts.push(s.as_str().to_string());
        }
        if let Some(s) = &self.region {
            parts.push(s.as_str().to_string());
        }
        if let Some(s) = &self.postal_code {
            parts.push(s.as_str().to_string());
        }
        if let Some(c) = &self.country {
            if let Ok(s) = core::str::from_utf8(c) {
                parts.push(s.to_string());
            }
        }
        write!(f, "{}", parts.join(", "))
    }
}

// ============================================================
// From / TryFrom — std::net ↔ VsfType ============================================================ Gated on `std` because `std::net::*` (IpAddr, Ipv4Addr, …) only exists in std.

#[cfg(feature = "std")]
impl From<Ipv4Addr> for VsfType {
    fn from(a: Ipv4Addr) -> Self {
        VsfType::ni(a.octets())
    }
}

#[cfg(feature = "std")]
impl From<Ipv6Addr> for VsfType {
    fn from(a: Ipv6Addr) -> Self {
        VsfType::nj(a.octets())
    }
}

#[cfg(feature = "std")]
impl From<IpAddr> for VsfType {
    fn from(a: IpAddr) -> Self {
        match a {
            IpAddr::V4(v4) => VsfType::ni(v4.octets()),
            IpAddr::V6(v6) => VsfType::nj(v6.octets()),
        }
    }
}

#[cfg(feature = "std")]
/// Socket address → `ns` (host:port, no scheme)
impl From<SocketAddrV4> for VsfType {
    fn from(a: SocketAddrV4) -> Self {
        VsfType::ns(a.ip().to_string(), a.port())
    }
}

#[cfg(feature = "std")]
impl From<SocketAddrV6> for VsfType {
    fn from(a: SocketAddrV6) -> Self {
        VsfType::ns(format!("[{}]", a.ip()), a.port())
    }
}

#[cfg(feature = "std")]
impl From<SocketAddr> for VsfType {
    fn from(a: SocketAddr) -> Self {
        match a {
            SocketAddr::V4(v4) => VsfType::from(v4),
            SocketAddr::V6(v6) => VsfType::from(v6),
        }
    }
}

/// Postal address → `wa`
impl From<WaAddress> for VsfType {
    fn from(a: WaAddress) -> Self {
        VsfType::wa(a)
    }
}

// ---- TryFrom<VsfType> ----

#[cfg(feature = "std")]
impl TryFrom<VsfType> for Ipv4Addr {
    type Error = &'static str;
    fn try_from(v: VsfType) -> Result<Self, Self::Error> {
        match v {
            VsfType::ni(b) => Ok(Ipv4Addr::from(b)),
            VsfType::nh(s) => s.parse().map_err(|_| "not a valid IPv4 string"),
            _ => Err("not an IPv4 type"),
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<VsfType> for Ipv6Addr {
    type Error = &'static str;
    fn try_from(v: VsfType) -> Result<Self, Self::Error> {
        match v {
            VsfType::nj(b) => Ok(Ipv6Addr::from(b)),
            VsfType::nh(s) => s
                .trim_matches(|c| c == '[' || c == ']')
                .parse()
                .map_err(|_| "not a valid IPv6 string"),
            _ => Err("not an IPv6 type"),
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<VsfType> for IpAddr {
    type Error = &'static str;
    fn try_from(v: VsfType) -> Result<Self, Self::Error> {
        match v {
            VsfType::ni(b) => Ok(IpAddr::V4(Ipv4Addr::from(b))),
            VsfType::nj(b) => Ok(IpAddr::V6(Ipv6Addr::from(b))),
            VsfType::nh(s) => s.parse().map_err(|_| "not a valid IP address string"),
            _ => Err("not an IP address type"),
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<VsfType> for SocketAddr {
    type Error = &'static str;
    fn try_from(v: VsfType) -> Result<Self, Self::Error> {
        match v {
            VsfType::ns(host, port) => {
                let addr_str = format!("{}:{}", host, port);
                addr_str.parse().map_err(|_| "not a valid socket address")
            }
            _ => Err("not a socket address type"),
        }
    }
}

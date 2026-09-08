//! `#[derive(Vsf)]` — one struct definition generates BOTH codec halves for a VSF section, killing the build/parse mirror-drift class and the positional-tuple churn (photon 2026-09-08).
//!
//! Field mapping is by Rust TYPE, refined by `#[vsf(...)]` attrs:
//! - `String` → text (`x`; `#[vsf(ascii)]` → `a`) · `u8..u64/usize` → auto-sized unsigned (read via as_u64 — width-agnostic, the ek-trap doctrine) · `i64` → auto-sized signed, or eagle-time `e6` with `#[vsf(eagle)]` · `bool` → flag · `[u8; N]`/`Vec<u8>` → bytes (the WIRE VARIANT must be named: `#[vsf(kind = "hb")]` etc. — byte semantics matter on the wire).
//! - `Option<T>` → optional (absent ⇒ None — the additive contract) · `Vec<T>` (non-u8) → repeated single-value fields in order.
//! - `#[vsf(name = "ftip")]` renames; `#[vsf(skip)]` leaves a field to hand-written code (rows with heterogeneous type-marker values stay manual).
//!
//! Readers are TOTAL for optional/repeated fields and `Err` only for a missing REQUIRED (non-Option, non-Vec, non-bool) field — fork-detector food, never a panic. Generated: `to_section(&self) -> VsfSection` and `from_section(&VsfSection) -> Result<Self, String>`. Field order in the struct IS the wire order (signatures cover bytes; order is part of the contract you declare).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Ts2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr, Type};

struct FieldAttrs {
    name: Option<String>,
    eagle: bool,
    ascii: bool,
    kind: Option<String>,
    skip: bool,
}

fn parse_attrs(attrs: &[syn::Attribute]) -> Result<FieldAttrs, syn::Error> {
    let mut out = FieldAttrs { name: None, eagle: false, ascii: false, kind: None, skip: false };
    for attr in attrs {
        if !attr.path().is_ident("vsf") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                out.name = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("kind") {
                out.kind = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("eagle") {
                out.eagle = true;
            } else if meta.path.is_ident("ascii") {
                out.ascii = true;
            } else if meta.path.is_ident("skip") {
                out.skip = true;
            } else if meta.path.is_ident("section") {
                let _ = meta.value()?.parse::<LitStr>()?; // struct-level, handled separately
            } else {
                return Err(meta.error("unknown vsf attr"));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn struct_section_name(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if !attr.path().is_ident("vsf") {
            continue;
        }
        let mut found = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                found = Some(meta.value()?.parse::<LitStr>()?.value());
            }
            Ok(())
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// What one Rust type means on the wire.
enum Kind {
    Text { ascii: bool },
    Uint,
    Int,
    Eagle,
    Flag,
    Bytes { kind: String, fixed: Option<usize> },
}

fn type_ident(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        p.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

/// Unwrap Option<T> / Vec<T> one level; returns (container, inner).
fn unwrap_container(ty: &Type) -> (Option<&'static str>, &Type) {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            let c = match seg.ident.to_string().as_str() {
                "Option" => Some("Option"),
                "Vec" => Some("Vec"),
                _ => None,
            };
            if let Some(c) = c {
                if let syn::PathArguments::AngleBracketed(a) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = a.args.first() {
                        return (Some(c), inner);
                    }
                }
            }
        }
    }
    (None, ty)
}

fn classify(inner: &Type, a: &FieldAttrs, field: &str) -> Result<Kind, String> {
    if let Type::Array(arr) = inner {
        let fixed = if let syn::Expr::Lit(l) = &arr.len {
            if let syn::Lit::Int(i) = &l.lit { i.base10_parse::<usize>().ok() } else { None }
        } else {
            None
        };
        let kind = a.kind.clone().ok_or_else(|| format!("field `{field}`: byte fields need #[vsf(kind = \"hb\")] (or hp/hg/hR/ke/ge/hb…) — the wire variant is semantics"))?;
        return Ok(Kind::Bytes { kind, fixed });
    }
    match type_ident(inner).as_deref() {
        Some("String") => Ok(Kind::Text { ascii: a.ascii }),
        Some("u8") | Some("u16") | Some("u32") | Some("u64") | Some("usize") => Ok(Kind::Uint),
        Some("i64") | Some("i32") | Some("isize") => {
            if a.eagle { Ok(Kind::Eagle) } else { Ok(Kind::Int) }
        }
        Some("bool") => Ok(Kind::Flag),
        Some("Vec") => {
            // Vec<u8> only (Vec<T> handled by the container layer before classify).
            let kind = a.kind.clone().ok_or_else(|| format!("field `{field}`: byte fields need #[vsf(kind = \"...\")]"))?;
            Ok(Kind::Bytes { kind, fixed: None })
        }
        other => Err(format!("field `{field}`: unsupported type {:?} — use #[vsf(skip)] and hand-write it", other)),
    }
}

fn wire_variant(kind: &str, value: Ts2) -> Ts2 {
    match kind {
        "hb" => quote! { ::vsf::VsfType::hb(#value) },
        "hp" => quote! { ::vsf::VsfType::hp(#value) },
        "hg" => quote! { ::vsf::VsfType::hg(#value) },
        "hR" => quote! { ::vsf::VsfType::hR(#value) },
        "ke" => quote! { ::vsf::VsfType::ke(#value) },
        "ge" => quote! { ::vsf::VsfType::ge(#value) },
        _ => quote! { compile_error!("unknown vsf byte kind") },
    }
}

#[proc_macro_derive(Vsf, attributes(vsf))]
pub fn derive_vsf(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();
    let section_name = struct_section_name(&input).unwrap_or_else(|| ident.to_string().to_lowercase());
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(n) => &n.named,
            _ => {
                return syn::Error::new_spanned(&input, "#[derive(Vsf)] needs named fields — that's the whole point")
                    .to_compile_error()
                    .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "#[derive(Vsf)] supports structs only")
                .to_compile_error()
                .into()
        }
    };

    let mut encodes: Vec<Ts2> = Vec::new();
    let mut decodes: Vec<Ts2> = Vec::new();

    for f in fields {
        let fid = f.ident.clone().unwrap();
        let fname = fid.to_string();
        let attrs = match parse_attrs(&f.attrs) {
            Ok(a) => a,
            Err(e) => return e.to_compile_error().into(),
        };
        if attrs.skip {
            decodes.push(quote! { let #fid = ::core::default::Default::default(); });
            continue;
        }
        let wire = attrs.name.clone().unwrap_or_else(|| fname.clone());
        let (container, inner) = unwrap_container(&f.ty);
        // Vec<u8> is bytes, not a repeated container.
        let container = if container == Some("Vec") && type_ident(inner).as_deref() == Some("u8") {
            None
        } else {
            container
        };
        let inner = if container.is_none() { &f.ty } else { inner };
        let kind = match classify(inner, &attrs, &fname) {
            Ok(k) => k,
            Err(msg) => {
                return syn::Error::new_spanned(f, msg).to_compile_error().into();
            }
        };

        // Per-kind fragments: (encode one value `v` into VsfType, decode one &VsfType into Option<T>).
        let (enc_one, dec_one): (Ts2, Ts2) = match &kind {
            Kind::Text { ascii } => {
                let e = if *ascii {
                    quote! { ::vsf::VsfType::a(v.clone()) }
                } else {
                    quote! { ::vsf::VsfType::x(v.clone()) }
                };
                (e, quote! { val.as_string().map(|s| s.to_string()) })
            }
            Kind::Uint => (
                quote! { ::vsf::VsfType::u(*v as usize, false) },
                quote! { val.as_u64().and_then(|n| ::core::convert::TryFrom::try_from(n).ok()) },
            ),
            Kind::Int => (
                quote! { ::vsf::VsfType::i(*v as isize) },
                quote! { val.as_i64().and_then(|n| ::core::convert::TryFrom::try_from(n).ok()) },
            ),
            Kind::Eagle => (
                quote! { ::vsf::VsfType::e(::vsf::types::EtType::e6(*v)) },
                quote! { val.as_eagle() },
            ),
            Kind::Flag => (
                quote! { ::vsf::VsfType::u0(*v) },
                quote! { Some(val.as_u64().is_some_and(|n| n != 0)) },
            ),
            Kind::Bytes { kind, fixed } => {
                let variant = wire_variant(kind, quote! { v.to_vec() });
                let dec = match fixed {
                    Some(n) => quote! { val.as_bytes().and_then(|b| <[u8; #n]>::try_from(b).ok()) },
                    None => quote! { val.as_bytes().map(|b| b.to_vec()) },
                };
                (variant, dec)
            }
        };

        match container {
            None => {
                if matches!(kind, Kind::Flag) {
                    // A flag encodes only when true (absent = false — the additive idiom).
                    encodes.push(quote! {
                        if self.#fid {
                            let v = &self.#fid;
                            section.add_field_multi(#wire, vec![#enc_one]);
                        }
                    });
                    decodes.push(quote! {
                        let #fid = section.flag(#wire);
                    });
                } else {
                    encodes.push(quote! {
                        {
                            let v = &self.#fid;
                            section.add_field_multi(#wire, vec![#enc_one]);
                        }
                    });
                    decodes.push(quote! {
                        let #fid = section
                            .get_field(#wire)
                            .and_then(|f| f.values.first())
                            .and_then(|val| #dec_one)
                            .ok_or_else(|| format!(concat!(#wire, " missing or mistyped")))?;
                    });
                }
            }
            Some("Option") => {
                encodes.push(quote! {
                    if let Some(v) = self.#fid.as_ref() {
                        section.add_field_multi(#wire, vec![#enc_one]);
                    }
                });
                decodes.push(quote! {
                    let #fid = section
                        .get_field(#wire)
                        .and_then(|f| f.values.first())
                        .and_then(|val| #dec_one);
                });
            }
            Some("Vec") => {
                encodes.push(quote! {
                    for v in self.#fid.iter() {
                        section.add_field_multi(#wire, vec![#enc_one]);
                    }
                });
                decodes.push(quote! {
                    let #fid = section
                        .get_fields(#wire)
                        .iter()
                        .filter_map(|f| f.values.first())
                        .filter_map(|val| #dec_one)
                        .collect();
                });
            }
            _ => unreachable!(),
        }
    }

    // Collect field idents for the constructor.
    let field_idents: Vec<syn::Ident> = fields.iter().map(|f| f.ident.clone().unwrap()).collect();
    let out = quote! {
        impl #ident {
            /// Encode into the section this struct declares — field order in the struct IS the wire order.
            pub fn to_section(&self) -> ::vsf::VsfSection {
                let mut section = ::vsf::VsfSection::new(#section_name);
                #(#encodes)*
                section
            }
            /// Decode from a parsed section. Optional/repeated/flag fields are TOTAL (absent ⇒ None/empty/false); a missing REQUIRED field is one Err — fork-detector food, never a panic.
            pub fn from_section(section: &::vsf::VsfSection) -> ::core::result::Result<Self, ::std::string::String> {
                #(#decodes)*
                Ok(Self { #(#field_idents),* })
            }
        }
    };
    out.into()
}

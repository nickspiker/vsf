//! Multi-scheme signature lists ("eggs"), and their one wire form.
//!
//! An egg is one signature under one scheme; a list of them is what a slot carries when it must hold several schemes at once — the rule being that EVERY listed egg must verify and the required schemes must all be listed. This module is only the shape and the codec: which schemes exist, what verifies them, and which set a context requires all live in the layers above (the fgtw crate's `pq` and `fleet`). vsf carries eggs the way it carries any other bytes, and checks exactly one thing itself — the Ed25519 egg, the anchor every reader can check with no chain and no extra crypto.
//!
//! The wire form is `u8 count`, then per egg `u8 scheme ‖ u32 LE len ‖ sig`. Every element framed, so the blob is safe inside a signing preimage; the same bytes ride the fleet chain's consent, the bindreq registry, the RustDesk handshake, the phonebook egg pointer and the VSF header signature (`gm`), so "egg-list shaped" is one codec everywhere rather than one per site.

/// One signature egg: which scheme, and the signature bytes. Scheme tags are wire-stable and append-only; `0` is Ed25519.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Egg {
    pub scheme: u8,
    pub sig: Vec<u8>,
}

/// The Ed25519 scheme tag — the one egg vsf itself can verify, and the one every device holds.
pub const SCHEME_ED25519: u8 = 0;

/// Encode an egg list.
pub fn eggs_to_bytes(eggs: &[Egg]) -> Vec<u8> {
    let mut v = Vec::with_capacity(1 + eggs.iter().map(|e| 5 + e.sig.len()).sum::<usize>());
    v.push(eggs.len() as u8);
    for e in eggs {
        v.push(e.scheme);
        v.extend_from_slice(&(e.sig.len() as u32).to_le_bytes());
        v.extend_from_slice(&e.sig);
    }
    v
}

/// Decode an egg list. Rejects an empty list (a slot with no signature is not a signed slot), duplicate schemes, and trailing bytes.
pub fn eggs_from_bytes(b: &[u8]) -> Result<Vec<Egg>, &'static str> {
    let Some((&count, mut rest)) = b.split_first() else {
        return Err("egg list: empty");
    };
    if count == 0 {
        return Err("egg list: no eggs");
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if rest.len() < 5 {
            return Err("egg list: truncated egg");
        }
        let scheme = rest[0];
        let n = u32::from_le_bytes([rest[1], rest[2], rest[3], rest[4]]) as usize;
        rest = &rest[5..];
        if rest.len() < n {
            return Err("egg list: truncated signature");
        }
        if out.iter().any(|e: &Egg| e.scheme == scheme) {
            return Err("egg list: duplicate scheme");
        }
        out.push(Egg { scheme, sig: rest[..n].to_vec() });
        rest = &rest[n..];
    }
    if !rest.is_empty() {
        return Err("egg list: trailing bytes");
    }
    Ok(out)
}

/// A zero-filled egg list with the given `(scheme, signature length)` slots — the placeholder a builder reserves so a signer can later patch real signatures in place. Byte-for-byte the same length as the signed form, which is what makes in-place patching sound: the file hash is computed over this exact layout with the whole value zeroed.
pub fn placeholder_eggs(slots: &[(u8, usize)]) -> Vec<u8> {
    let eggs: Vec<Egg> = slots.iter().map(|(s, n)| Egg { scheme: *s, sig: vec![0u8; *n] }).collect();
    eggs_to_bytes(&eggs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_malformed() {
        let eggs = vec![Egg { scheme: 0, sig: vec![1u8; 64] }, Egg { scheme: 1, sig: vec![2u8; 666] }];
        assert_eq!(eggs_from_bytes(&eggs_to_bytes(&eggs)).unwrap(), eggs);
        assert!(eggs_from_bytes(&[]).is_err());
        assert!(eggs_from_bytes(&[0]).is_err());
        let mut dup = eggs.clone();
        dup.push(Egg { scheme: 0, sig: vec![3u8; 64] });
        assert!(eggs_from_bytes(&eggs_to_bytes(&dup)).is_err());
        let mut t = eggs_to_bytes(&eggs);
        t.push(0);
        assert!(eggs_from_bytes(&t).is_err());
    }

    #[test]
    fn placeholder_matches_signed_length() {
        let ph = placeholder_eggs(&[(0, 64), (1, 666)]);
        let signed = eggs_to_bytes(&[Egg { scheme: 0, sig: vec![9u8; 64] }, Egg { scheme: 1, sig: vec![9u8; 666] }]);
        assert_eq!(ph.len(), signed.len());
    }
}

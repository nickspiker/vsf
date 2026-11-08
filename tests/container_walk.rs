//! A large section located through a window (the container's walk) agrees with the whole walk, and never reads its bulk payload.

use std::cell::Cell;
use vsf::container::{ByteSource, Document, SourceError};

/// A source with no in-memory slice, as a vault or a socket is: every byte comes through `read_at`, and the largest read is kept.
struct Remote {
    bytes: Vec<u8>,
    largest: Cell<usize>,
}

impl ByteSource for Remote {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), SourceError> {
        self.largest.set(self.largest.get().max(buf.len()));
        buf.copy_from_slice(&self.bytes[offset as usize..offset as usize + buf.len()]);
        Ok(())
    }
}

#[test]
fn large_plane_located_without_reading_it() {
    let (w, h) = (640usize, 480usize);
    let samples: Vec<u16> = (0..3 * w * h).map(|i| (i * 7919) as u16).collect();
    let tensor = vsf::types::BitPackedTensor::pack(16, vec![3, h, w], &samples);
    let mut plane = vsf::visual::Plane::encoded("display", "wallpaper", w, h, vsf::visual::Primaries::Vsf, vsf::visual::Encoding::Gamma2Tanh, tensor);
    plane.name = "main".into();
    let v = vsf::visual::Visual { title: "t".into(), text: "t".into(), planes: vec![plane], ..Default::default() };
    let bytes = v.write().unwrap();

    let whole = Document::open(bytes.clone()).unwrap();
    let e = whole.entries().iter().find(|e| e.name.starts_with("plane.")).unwrap().clone();
    let a = whole.packed(&e, "samples").unwrap().unwrap();

    let remote = Remote { bytes: bytes.clone(), largest: Cell::new(0) };
    let doc = Document::open(&remote).unwrap();
    let e = doc.entries().iter().find(|e| e.name.starts_with("plane.")).unwrap().clone();
    let b = doc.packed(&e, "samples").unwrap().unwrap();
    assert_eq!(a, b);
    assert_eq!(doc.fields(&e).unwrap(), whole.fields(&e).unwrap());
    // The plane is 1.8 MB; the walk read a window, never the plane.
    assert!(remote.largest.get() <= 1 << 16, "largest read {}", remote.largest.get());
    // And the rows read through it are the samples.
    assert_eq!(doc.packed_rows_u16(&b, 0, 1).unwrap(), samples[..w * h].to_vec());
}

use std::sync::Arc;

use crate::pipeline::build::BuildBoundaryId;

/// Addresses one dispatch destination of a routing widget.
///
/// An id stays with the element it addresses for that element's whole life: a widget that reorders its
/// children keeps each child's id, so a path captured before the reorder still reaches the same element.
/// An id is never reissued while a path may still hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingId(u32);

impl RoutingId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns this id and advances `self` past it, so the next call yields a fresh one. Used to mint a
    /// never-reissued id per child.
    pub const fn next(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    /// Appends this id to `buf` in the routing encoding: the leading one-bits of the first byte give how
    /// many bytes follow, so an id below 128 takes one byte and a larger one grows a byte at a time.
    #[allow(clippy::cast_possible_truncation)] // each cast keeps the low byte of a shift, by design
    pub fn encode(self, buf: &mut Vec<u8>) {
        let value = u64::from(self.0);

        let following = match self.0 {
            0..=0x7F => 0,
            0x80..=0x3FFF => 1,
            0x4000..=0x001F_FFFF => 2,
            0x0020_0000..=0x0FFF_FFFF => 3,
            _ => 4,
        };

        // The high `following` bits of the first byte are ones, the next bit the terminating zero.
        let marker = !(0xFF_u8 >> following);
        buf.push(marker | (value >> (8 * following)) as u8);
        for shift in (0..following).rev() {
            buf.push((value >> (8 * shift)) as u8);
        }
    }

    /// Encodes a sequence of ids into one routing buffer, the bytes a [`RoutingPath`] borrows.
    pub fn encode_path(ids: impl IntoIterator<Item = RoutingId>) -> Vec<u8> {
        let mut buf = Vec::new();

        for id in ids {
            id.encode(&mut buf);
        }

        buf
    }
}

/// A sequence of routing ids in the variable-length encoding, addressing a descendant relative to the
/// element holding it. Borrowed like [`str`] is from a `String`: [`decode`](Self::decode) peels the
/// leading id and returns the rest of the path, so each routing widget reads one id and forwards the
/// tail.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RoutingPath([u8]);

impl RoutingPath {
    /// Borrows `bytes` as a routing path.
    #[allow(clippy::ptr_as_ptr)] // slice-to-newtype is a fat-pointer cast, so `.cast()` does not apply
    pub fn new(bytes: &[u8]) -> &RoutingPath {
        // SAFETY: `RoutingPath` is `repr(transparent)` over `[u8]`, so the references are interchangeable.
        unsafe { &*(std::ptr::from_ref::<[u8]>(bytes) as *const RoutingPath) }
    }

    /// Whether the path addresses the element holding it rather than a descendant.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The encoded bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Peels the leading id, returning it with the rest of the path, or `None` if the path is empty or
    /// ends mid-id.
    pub fn decode(&self) -> Option<(RoutingId, &RoutingPath)> {
        let (&first, rest) = self.0.split_first()?;

        let following = first.leading_ones() as usize;
        if following > 4 || rest.len() < following {
            return None;
        }

        // A five-byte id has no payload bits to spare in its marker; one set would put the value
        // past `u32`.
        if following == 4 && first & 0x07 != 0 {
            return None;
        }

        let mut value = u32::from(first & (0xFF >> (following + 1)));
        let (extra, tail) = rest.split_at(following);
        for &byte in extra {
            value = (value << 8) | u32::from(byte);
        }

        // The smallest value needing each byte count; a value below its encoding's threshold is an
        // overlong form `encode` never writes.
        #[allow(clippy::items_after_statements)]
        const MIN_FOR_LEN: [u32; 5] = [0, 0x80, 0x4000, 0x0020_0000, 0x1000_0000];
        if value < MIN_FOR_LEN[following] {
            return None;
        }

        Some((RoutingId(value), RoutingPath::new(tail)))
    }

    /// Whether the whole buffer decodes into ids end to end, with no trailing partial id.
    pub fn is_well_formed(&self) -> bool {
        let mut rest = self;
        while !rest.is_empty() {
            let Some((_, tail)) = rest.decode() else {
                return false;
            };
            rest = tail;
        }
        true
    }
}

/// Addresses one element for dispatch: the build boundary it lives under, and the [`RoutingPath`] from
/// that boundary's inner element down to it.
///
/// A target is resolved by looking its boundary up in the registry and decoding the path within, so
/// dispatch reaches the element without descending from the root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingTarget {
    boundary: BuildBoundaryId,
    path: Arc<[u8]>,
}

impl RoutingTarget {
    /// Addresses the element reached by `path` from `boundary`'s inner element.
    ///
    /// # Panics
    ///
    /// Panics if `path` is not a well-formed routing encoding.
    pub fn new(boundary: BuildBoundaryId, path: impl Into<Arc<[u8]>>) -> Self {
        let path = path.into();
        assert!(
            RoutingPath::new(&path).is_well_formed(),
            "routing target built from a malformed path"
        );
        Self { boundary, path }
    }

    /// Addresses the element reached by `path` from `boundary`'s inner element, trusting `path` to be a
    /// well-formed routing encoding.
    pub(crate) fn new_unchecked(boundary: BuildBoundaryId, path: impl Into<Arc<[u8]>>) -> Self {
        Self {
            boundary,
            path: path.into(),
        }
    }

    /// The boundary this target is relative to.
    pub fn boundary(&self) -> BuildBoundaryId {
        self.boundary
    }

    /// The path from the boundary's inner element down to the addressed element.
    pub fn path(&self) -> &RoutingPath {
        RoutingPath::new(&self.path)
    }

    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::pipeline::build::BuildBoundaryId;

    use super::{RoutingId, RoutingPath, RoutingTarget};

    /// The id values at and around each byte-length boundary of the encoding.
    const BOUNDARIES: [u32; 13] = [
        0,
        1,
        127,
        128,
        16_383,
        16_384,
        2_097_151,
        2_097_152,
        268_435_455,
        268_435_456,
        u32::MAX - 1,
        u32::MAX,
        0xDEAD_BEEF,
    ];

    fn encode(value: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        RoutingId::new(value).encode(&mut buf);
        buf
    }

    #[test]
    fn encodes_each_magnitude_in_the_expected_byte_count() {
        assert_eq!(encode(0).len(), 1);
        assert_eq!(encode(127).len(), 1);
        assert_eq!(encode(128).len(), 2);
        assert_eq!(encode(16_383).len(), 2);
        assert_eq!(encode(16_384).len(), 3);
        assert_eq!(encode(2_097_151).len(), 3);
        assert_eq!(encode(2_097_152).len(), 4);
        assert_eq!(encode(268_435_455).len(), 4);
        assert_eq!(encode(268_435_456).len(), 5);
        assert_eq!(encode(u32::MAX).len(), 5);
    }

    #[test]
    fn decode_recovers_each_boundary_value_and_consumes_its_bytes() {
        for value in BOUNDARIES {
            let buf = encode(value);
            let (id, rest) = RoutingPath::new(&buf).decode().expect("a whole id decodes");
            assert_eq!(id.get(), value);
            assert!(rest.is_empty(), "the whole buffer was one id");
        }
    }

    #[test]
    fn decode_walks_a_sequence_in_order() {
        let ids = [7, 200, 1_000_000, 3, u32::MAX];
        let buf = RoutingId::encode_path(ids.map(RoutingId::new));

        let mut rest = RoutingPath::new(&buf);
        for expected in ids {
            let (id, tail) = rest.decode().expect("another id remains");
            assert_eq!(id.get(), expected);
            rest = tail;
        }
        assert!(rest.is_empty(), "the sequence was fully consumed");
    }

    #[test]
    fn decode_rejects_empty_and_truncated_buffers() {
        assert!(RoutingPath::new(&[]).decode().is_none());

        // A two-byte id missing its trailing byte.
        let mut buf = encode(128);
        buf.pop();
        assert!(RoutingPath::new(&buf).decode().is_none());
    }

    #[test]
    fn decode_rejects_overlong_encodings() {
        // Id 5 in two and three bytes; its canonical encoding is the single byte 0x05.
        assert!(RoutingPath::new(&[0x80, 0x05]).decode().is_none());
        assert!(RoutingPath::new(&[0xC0, 0x00, 0x05]).decode().is_none());

        // The smallest value of each byte count is canonical and still decodes.
        for value in [0x80, 0x4000, 0x0020_0000, 0x1000_0000] {
            let buf = encode(value);
            assert_eq!(RoutingPath::new(&buf).decode().unwrap().0.get(), value);
        }
    }

    #[test]
    fn decode_rejects_a_five_byte_marker_with_payload_bits() {
        // Payload bits in a five-byte marker would encode a value past u32; accepting them would
        // silently shift the bits out instead.
        assert!(
            RoutingPath::new(&[0xF1, 0xAA, 0xBB, 0xCC, 0xDD])
                .decode()
                .is_none()
        );
        assert!(!RoutingPath::new(&[0xF7, 0x00, 0x00, 0x00, 0x00]).is_well_formed());
    }

    #[test]
    fn well_formed_accepts_whole_sequences_and_rejects_a_truncated_tail() {
        let buf = RoutingId::encode_path([7, 128, 70_000].map(RoutingId::new));
        assert!(RoutingPath::new(&buf).is_well_formed());
        assert!(RoutingPath::new(&[]).is_well_formed());

        let mut truncated = buf;
        truncated.pop();
        assert!(!RoutingPath::new(&truncated).is_well_formed());
    }

    #[test]
    #[should_panic(expected = "malformed path")]
    fn routing_target_rejects_a_malformed_path() {
        let mut buf = encode(128);
        buf.pop();
        RoutingTarget::new(BuildBoundaryId::default(), buf);
    }
}

//! Binary layout, encoding, and decoding for RailID v1.

use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use crate::RailIdError;

/// Total number of bits in a RailID v1 value.
pub const BITS: usize = 192;
/// Number of bytes in a RailID v1 value.
pub const BINARY_LEN: usize = 24;
/// Number of characters in an encoded RailID v1 value.
pub const ENCODED_LEN: usize = 32;
/// Number of timestamp bits.
pub const TIMESTAMP_BITS: u32 = 42;
/// Number of random bits.
pub const RANDOM_BITS: u32 = 150;
/// Number of bytes acquired from each randomness source.
pub const RANDOM_ACQUIRE_BYTES: usize = 19;
/// Maximum value of the 42-bit timestamp field.
pub const TIMESTAMP_MAX: u64 = (1u64 << TIMESTAMP_BITS) - 1;
/// Timestamp ticks per second (100 ms precision).
pub const TICKS_PER_SECOND: u64 = 10;

const _: () = assert!(BINARY_LEN * 8 == BITS);
const _: () = assert!(TIMESTAMP_BITS + RANDOM_BITS == BITS as u32);
const _: () = assert!(RANDOM_ACQUIRE_BYTES * 8 == RANDOM_BITS as usize + 2);

const BASE64URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// A fixed-size Base64URL encoding of a [`RailId`].
///
/// This is stack-allocated and can be used without a heap allocation. It is
/// only constructed from a valid `RailId`, so `as_str` cannot fail.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EncodedRailId([u8; ENCODED_LEN]);

impl EncodedRailId {
    /// Borrow the encoded value as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Every byte comes from BASE64URL_ALPHABET, which is ASCII.
        core::str::from_utf8(&self.0).expect("RailID encoding is ASCII")
    }

    /// Borrow the raw encoded bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; ENCODED_LEN] {
        &self.0
    }
}

impl fmt::Display for EncodedRailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for EncodedRailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EncodedRailId({})", self.as_str())
    }
}

impl AsRef<str> for EncodedRailId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for EncodedRailId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// A contiguous batch of Base64URL-encoded IDs.
///
/// The batch owns one byte buffer of `len() * 32` bytes instead of one
/// `String` per ID. Individual entries are borrowed as string slices.
#[derive(Clone, PartialEq, Eq)]
pub struct EncodedBatch {
    data: Vec<u8>,
}

impl EncodedBatch {
    pub(crate) fn from_data(data: Vec<u8>) -> Self {
        debug_assert_eq!(data.len() % ENCODED_LEN, 0);
        Self { data }
    }

    /// Number of IDs in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len() / ENCODED_LEN
    }

    /// Whether the batch contains no IDs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Borrow one encoded ID.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        let start = index.checked_mul(ENCODED_LEN)?;
        let end = start.checked_add(ENCODED_LEN)?;
        self.data.get(start..end).map(|bytes| {
            // The buffer is only produced by the Base64URL encoder.
            core::str::from_utf8(bytes).expect("RailID encoding is ASCII")
        })
    }

    /// Iterate over encoded IDs in batch order.
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        (0..self.len()).map(|index| self.get(index).expect("index in batch"))
    }

    /// Borrow the raw encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consume the batch and return the contiguous encoded bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}

impl fmt::Debug for EncodedBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedBatch")
            .field("len", &self.len())
            .finish()
    }
}

/// A 192-bit RailID v1 identifier.
///
/// The binary representation stores the 42-bit timestamp first, followed by the
/// 150-bit random field, both in big-endian (MSB-first) order. A `RailId` is
/// fixed-length, `Copy`, and orders chronologically when compared as a byte
/// array because the timestamp is the most significant field.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailId([u8; BINARY_LEN]);

impl RailId {
    /// Assemble a RailID from a 42-bit timestamp tick count and a 150-bit
    /// random value.
    ///
    /// `random` is interpreted as a big-endian 150-bit integer: only the low
    /// 150 bits are used and the two most significant bits of `random[0]` are
    /// ignored. This is the representation returned by [`RailId::random`].
    ///
    /// # Errors
    ///
    /// Returns [`RailIdError::TimestampOutOfRange`] when `timestamp_ticks`
    /// exceeds 42 bits.
    pub fn from_parts(
        timestamp_ticks: u64,
        random: [u8; RANDOM_ACQUIRE_BYTES],
    ) -> Result<Self, RailIdError> {
        if timestamp_ticks > TIMESTAMP_MAX {
            return Err(RailIdError::TimestampOutOfRange);
        }

        let mut bytes = [0u8; BINARY_LEN];

        // Timestamp: bytes 0..=4 hold bits 41..=2, byte 5 bits 7..=6 hold bits 1..=0.
        bytes[0] = (timestamp_ticks >> 34) as u8;
        bytes[1] = (timestamp_ticks >> 26) as u8;
        bytes[2] = (timestamp_ticks >> 18) as u8;
        bytes[3] = (timestamp_ticks >> 10) as u8;
        bytes[4] = (timestamp_ticks >> 2) as u8;

        // Random: byte 5 bits 5..=0 hold the top 6 bits, bytes 6..=23 hold the rest.
        bytes[5] = (((timestamp_ticks & 0x3) as u8) << 6) | (random[0] & 0x3f);
        bytes[6..].copy_from_slice(&random[1..]);

        Ok(Self(bytes))
    }

    /// Wrap 24 raw bytes without validation.
    ///
    /// The caller is responsible for supplying bytes in the RailID v1 layout.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; BINARY_LEN]) -> Self {
        Self(bytes)
    }

    /// Return the 24-byte binary representation.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; BINARY_LEN] {
        self.0
    }

    /// Borrow the 24-byte binary representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; BINARY_LEN] {
        &self.0
    }

    /// Return the timestamp as 100 ms ticks since the Unix Epoch.
    #[must_use]
    pub fn timestamp(&self) -> u64 {
        ((self.0[0] as u64) << 34)
            | ((self.0[1] as u64) << 26)
            | ((self.0[2] as u64) << 18)
            | ((self.0[3] as u64) << 10)
            | ((self.0[4] as u64) << 2)
            | ((self.0[5] as u64) >> 6)
    }

    /// Return the timestamp as Unix milliseconds.
    ///
    /// This is `timestamp() * 100`. No timezone conversion is involved.
    #[must_use]
    pub fn timestamp_millis(&self) -> u64 {
        self.timestamp() * 100
    }

    /// Return the 150-bit random field as 19 big-endian bytes.
    ///
    /// The value is right-aligned: the two most significant bits of byte 0 are
    /// always zero because only 150 of the 152 bits in 19 bytes are valid.
    #[must_use]
    pub fn random(&self) -> [u8; RANDOM_ACQUIRE_BYTES] {
        let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
        random[0] = self.0[5] & 0x3f;
        random[1..].copy_from_slice(&self.0[6..]);
        random
    }

    /// Encode as a stack-allocated 32-character unpadded Base64URL value.
    #[must_use]
    pub fn encode_array(&self) -> EncodedRailId {
        let mut output = [0u8; ENCODED_LEN];
        for group in 0..(BINARY_LEN / 3) {
            let source = group * 3;
            let target = group * 4;
            let first = self.0[source];
            let second = self.0[source + 1];
            let third = self.0[source + 2];

            output[target] = BASE64URL_ALPHABET[usize::from(first >> 2)];
            output[target + 1] =
                BASE64URL_ALPHABET[usize::from(((first & 0x03) << 4) | (second >> 4))];
            output[target + 2] =
                BASE64URL_ALPHABET[usize::from(((second & 0x0f) << 2) | (third >> 6))];
            output[target + 3] = BASE64URL_ALPHABET[usize::from(third & 0x3f)];
        }
        EncodedRailId(output)
    }

    /// Encode as 32-character unpadded Base64URL.
    #[must_use]
    pub fn encode(&self) -> String {
        self.encode_array().as_str().to_owned()
    }

    /// Decode a 32-character unpadded Base64URL RailID.
    ///
    /// # Errors
    ///
    /// Returns [`RailIdError::InvalidLength`] unless the input is exactly 32
    /// bytes, and [`RailIdError::InvalidEncoding`] for non-BaseURL input,
    /// padding, or non-canonical encodings.
    pub fn decode(input: &str) -> Result<Self, RailIdError> {
        if input.len() != ENCODED_LEN {
            return Err(RailIdError::InvalidLength);
        }

        let bytes = decode_base64url(input.as_bytes())?;
        let id = Self(bytes);

        // Reject non-canonical spellings. For 24-byte payloads there are no
        // unused trailing bits, but this keeps the check explicit and future-proof.
        if id.encode() != input {
            return Err(RailIdError::InvalidEncoding);
        }

        Ok(id)
    }
}

fn decode_base64url(input: &[u8]) -> Result<[u8; BINARY_LEN], RailIdError> {
    if input.len() != ENCODED_LEN {
        return Err(RailIdError::InvalidLength);
    }

    let mut output = [0u8; BINARY_LEN];
    for (index, chunk) in input.chunks_exact(4).enumerate() {
        let first = u32::from(decode_base64url_char(chunk[0])?);
        let second = u32::from(decode_base64url_char(chunk[1])?);
        let third = u32::from(decode_base64url_char(chunk[2])?);
        let fourth = u32::from(decode_base64url_char(chunk[3])?);
        let value = (first << 18) | (second << 12) | (third << 6) | fourth;

        let start = index * 3;
        output[start] = (value >> 16) as u8;
        output[start + 1] = (value >> 8) as u8;
        output[start + 2] = value as u8;
    }

    Ok(output)
}

fn decode_base64url_char(byte: u8) -> Result<u8, RailIdError> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'-' => Ok(62),
        b'_' => Ok(63),
        _ => Err(RailIdError::InvalidEncoding),
    }
}

impl fmt::Display for RailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl fmt::Debug for RailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RailId({})", self.encode())
    }
}

impl FromStr for RailId {
    type Err = RailIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::decode(s)
    }
}

/// Assemble a RailID from a timestamp tick count and a 150-bit random value.
///
/// # Errors
///
/// See [`RailId::from_parts`].
pub fn from_parts(
    timestamp_ticks: u64,
    random: [u8; RANDOM_ACQUIRE_BYTES],
) -> Result<RailId, RailIdError> {
    RailId::from_parts(timestamp_ticks, random)
}

/// Encode a RailID as unpadded Base64URL.
#[must_use]
pub fn encode(id: &RailId) -> String {
    id.encode()
}

/// Encode a RailID into a stack-allocated fixed-size value.
#[must_use]
pub fn encode_array(id: &RailId) -> EncodedRailId {
    id.encode_array()
}

/// Decode an unpadded Base64URL RailID.
///
/// # Errors
///
/// See [`RailId::decode`].
pub fn decode(input: &str) -> Result<RailId, RailIdError> {
    RailId::decode(input)
}

/// Return the timestamp in 100 ms ticks since the Unix Epoch.
#[must_use]
pub fn timestamp(id: &RailId) -> u64 {
    id.timestamp()
}

/// Return the 150-bit random field as 19 big-endian bytes.
#[must_use]
pub fn random(id: &RailId) -> [u8; RANDOM_ACQUIRE_BYTES] {
    id.random()
}

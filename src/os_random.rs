//! Bounded OS-randomness buffer and exact 150-bit extraction.
//!
//! RailID's 150-bit random field is not byte-aligned. For bulk OS-only
//! generation this module keeps a small OS CSPRNG buffer and consumes exactly
//! 150 bits per ID, so the two excess bits are not wasted and `getrandom` is
//! called once per buffer instead of once per ID.

use crate::{RailIdError, OS_RANDOM_BUFFER_BYTES, RANDOM_ACQUIRE_BYTES};

/// Size of one 152-bit (19-byte) staging buffer.
const STAGE_BYTES: usize = RANDOM_ACQUIRE_BYTES;

pub(crate) struct OsBitReader {
    buffer: Vec<u8>,
    len: usize,
    pos: usize,
    bit: u8,
}

impl OsBitReader {
    pub(crate) fn new() -> Self {
        Self {
            buffer: vec![0u8; OS_RANDOM_BUFFER_BYTES],
            len: 0,
            pos: 0,
            bit: 0,
        }
    }

    /// Consume exactly 150 bits and return the right-aligned value in 19 bytes.
    pub(crate) fn next_150(&mut self) -> Result<[u8; RANDOM_ACQUIRE_BYTES], RailIdError> {
        self.next_150_with(|buffer| {
            getrandom::fill(buffer).map_err(|_| RailIdError::OsRandomFailure)
        })
    }

    fn next_150_with<F>(&mut self, mut fill: F) -> Result<[u8; RANDOM_ACQUIRE_BYTES], RailIdError>
    where
        F: FnMut(&mut [u8]) -> Result<(), RailIdError>,
    {
        self.ensure_with(20, &mut fill)?;

        let offset = usize::from(self.bit);
        let source = &self.buffer[self.pos..self.pos + 20];
        let mut left_aligned = [0u8; STAGE_BYTES];

        if offset == 0 {
            left_aligned.copy_from_slice(&source[..STAGE_BYTES]);
        } else {
            for index in 0..STAGE_BYTES {
                left_aligned[index] =
                    (source[index] << offset) | (source[index + 1] >> (8 - offset));
            }
        }

        let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
        let mut carry = 0u8;
        for (index, byte) in left_aligned.iter().enumerate() {
            random[index] = (byte >> 2) | (carry << 6);
            carry = byte & 0x03;
        }

        // 150 bits = 18 bytes + 6 bits.
        self.pos += 18;
        self.bit += 6;
        if self.bit >= 8 {
            self.bit -= 8;
            self.pos += 1;
        }

        Ok(random)
    }

    fn ensure_with<F>(&mut self, needed: usize, fill: &mut F) -> Result<(), RailIdError>
    where
        F: FnMut(&mut [u8]) -> Result<(), RailIdError>,
    {
        if self.pos + needed <= self.len {
            return Ok(());
        }

        let leftover = self.len - self.pos;
        self.buffer.copy_within(self.pos..self.len, 0);
        self.len = leftover;
        self.pos = 0;

        fill(&mut self.buffer[self.len..])?;
        self.len = self.buffer.len();

        if self.len < needed {
            return Err(RailIdError::OsRandomFailure);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_once(source: &[u8; 20], bit: u8) -> [u8; RANDOM_ACQUIRE_BYTES] {
        let mut left_aligned = [0u8; STAGE_BYTES];
        let offset = usize::from(bit);
        if offset == 0 {
            left_aligned.copy_from_slice(&source[..STAGE_BYTES]);
        } else {
            for index in 0..STAGE_BYTES {
                left_aligned[index] =
                    (source[index] << offset) | (source[index + 1] >> (8 - offset));
            }
        }

        let mut random = [0u8; RANDOM_ACQUIRE_BYTES];
        let mut carry = 0u8;
        for (index, byte) in left_aligned.iter().enumerate() {
            random[index] = (byte >> 2) | (carry << 6);
            carry = byte & 0x03;
        }
        random
    }

    /// Slow bit-by-bit reference for the 150-bit extraction.
    fn reference(source: &[u8], bit_offset: usize) -> [u8; RANDOM_ACQUIRE_BYTES] {
        let mut output = [0u8; RANDOM_ACQUIRE_BYTES];
        for bit in 0..150 {
            let source_bit = bit_offset + bit;
            let value = (source[source_bit / 8] >> (7 - source_bit % 8)) & 1;
            let target = 149 - bit;
            output[18 - target / 8] |= value << (target % 8);
        }
        output
    }

    #[test]
    fn all_ones_produces_150_set_bits() {
        let source = [0xffu8; 20];
        let expected = {
            let mut value = [0xffu8; RANDOM_ACQUIRE_BYTES];
            value[0] = 0x3f;
            value
        };
        for bit in 0..8 {
            assert_eq!(extract_once(&source, bit), expected, "bit offset {bit}");
        }
    }

    #[test]
    fn matches_bit_by_bit_reference_at_every_offset() {
        let source: [u8; 20] = [
            0x00, 0x01, 0x7f, 0x80, 0xaa, 0x55, 0xde, 0xad, 0xbe, 0xef, 0x10, 0x20, 0x30, 0x40,
            0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
        ];
        for bit in 0..8 {
            assert_eq!(
                extract_once(&source, bit),
                reference(&source, usize::from(bit)),
                "bit offset {bit}"
            );
        }
    }

    #[test]
    fn reader_advances_by_exactly_150_bits() {
        let data: Vec<u8> = (0..256).map(|index| (index * 37) as u8).collect();
        let mut reader = OsBitReader {
            buffer: data.clone(),
            len: data.len(),
            pos: 0,
            bit: 0,
        };

        for id in 0..4 {
            let expected = reference(&data, id * 150);
            assert_eq!(reader.next_150().unwrap(), expected, "ID {id}");
        }
    }

    #[test]
    fn refills_preserve_bit_continuity() {
        let data: Vec<u8> = (0..4096).map(|index| (index * 73 + 19) as u8).collect();
        let mut cursor = 0usize;
        let mut fill = |destination: &mut [u8]| {
            let end = cursor
                .checked_add(destination.len())
                .expect("test stream exhausted");
            destination.copy_from_slice(&data[cursor..end]);
            cursor = end;
            Ok(())
        };

        // A 23-byte buffer forces a refill every two IDs, so the carry across
        // the buffer boundary is exercised at every bit offset.
        let mut reader = OsBitReader {
            buffer: vec![0u8; 23],
            len: 0,
            pos: 0,
            bit: 0,
        };

        for id in 0..200 {
            let expected = reference(&data, id * 150);
            let actual = reader.next_150_with(&mut fill).expect("extraction");
            assert_eq!(actual, expected, "ID {id}");
        }
    }
}

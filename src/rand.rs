//! Cryptographically secure randomness acquisition and hybrid combination.

use crate::tpm;
use crate::RailIdError;

/// Number of bytes acquired from each entropy source for one RailID.
pub const ACQUIRE_BYTES: usize = crate::id::RANDOM_ACQUIRE_BYTES;

/// A source of the two independent entropy inputs required by the TPM profile.
///
/// Implementations are primarily useful for deterministic tests and for
/// embedding RailID with an alternative TPM transport. Production code should
/// use [`SystemRandomSource`].
pub trait RandomSource {
    /// Fill `out` with cryptographically secure random bytes from the TPM 2.0
    /// RNG (`TPM2_GetRandom`).
    ///
    /// Implementations must handle partial responses and must never substitute
    /// another entropy source on failure.
    fn fill_tpm(&mut self, out: &mut [u8]) -> Result<(), RailIdError>;

    /// Fill `out` with cryptographically secure random bytes from the operating
    /// system CSPRNG.
    fn fill_os(&mut self, out: &mut [u8]) -> Result<(), RailIdError>;
}

/// The system TPM 2.0 device and operating system CSPRNG.
///
/// Without the `tpm` feature, the TPM half returns `TpmUnavailable`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRandomSource;

impl RandomSource for SystemRandomSource {
    fn fill_tpm(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        tpm::fill(out)
    }

    fn fill_os(&mut self, out: &mut [u8]) -> Result<(), RailIdError> {
        getrandom::fill(out).map_err(|_| RailIdError::OsRandomFailure)
    }
}

/// XOR two 19-byte buffers and retain the 150 most significant bits.
///
/// The result is a right-aligned 150-bit value in 19 bytes, so the top two bits
/// of byte 0 are always zero. The two excess least-significant bits are
/// discarded after the XOR; this is an unbiased reduction of a uniform 152-bit
/// value.
pub(crate) fn combine_hybrid(
    tpm_random: &[u8; ACQUIRE_BYTES],
    os_random: &[u8; ACQUIRE_BYTES],
) -> [u8; ACQUIRE_BYTES] {
    let mut random = [0u8; ACQUIRE_BYTES];
    let mut carry = 0u8;

    for (index, output) in random.iter_mut().enumerate() {
        let mixed = tpm_random[index] ^ os_random[index];
        *output = (mixed >> 2) | (carry << 6);
        carry = mixed & 0x03;
    }

    random
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_reduction_keeps_upper_150_bits() {
        let mut tpm = [0u8; ACQUIRE_BYTES];
        let os = [0u8; ACQUIRE_BYTES];
        tpm[0] = 0xff;
        tpm[18] = 0xff;

        let random = combine_hybrid(&tpm, &os);

        // 0xff at byte 0 contributes 6 bits plus two bits carried into byte 1.
        assert_eq!(random[0], 0x3f);
        assert_eq!(random[1], 0xc0);
        // The two lowest bits of the final byte are discarded.
        assert_eq!(random[18], 0x3f);
    }

    #[test]
    fn hybrid_reduction_is_xor_before_truncation() {
        let tpm = [0x55u8; ACQUIRE_BYTES];
        let os = [0xaau8; ACQUIRE_BYTES];

        // XOR is 0xff everywhere, then reduced to 150 bits.
        let random = combine_hybrid(&tpm, &os);
        assert_eq!(random[0], 0x3f);
        assert_eq!(random[1], 0xff);
        assert_eq!(random[18], 0xff);
    }
}

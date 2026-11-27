//! # RailID v1
//!
//! RailID is a fixed-length, URL-safe, time-aware identifier. A v1 RailID is
//! 192 bits: a 42-bit Unix timestamp in 100 ms ticks followed by 150 bits of
//! cryptographically secure randomness. It is encoded as 32 characters of
//! unpadded Base64URL.
//!
//! The default [`generate`] profile is a hybrid of the TPM 2.0 hardware RNG
//! (TSS ESAPI on Linux, TBS on Windows) and the operating system CSPRNG. The
//! two 19-byte inputs are XORed and reduced to 150 bits. If either source
//! fails, generation fails: the standard profile never silently downgrades to
//! OS-only randomness.
//!
//! ```
//! # fn main() -> Result<(), railid::RailIdError> {
//! let id = railid::RailId::from_parts(0, [0u8; 19])?;
//! assert_eq!(id.to_string(), "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
//! assert_eq!(id.timestamp(), 0);
//! assert_eq!(id.timestamp_millis(), 0);
//! # Ok(())
//! # }
//! ```
//!
//! ## Profiles
//!
//! * [`Profile::TpmHybrid`] — the conforming TPM 2.0 profile (default).
//! * [`Profile::OsOnlyDegraded`] — an explicitly selected degraded profile that
//!   uses only the OS CSPRNG. It is not a substitute for the TPM profile.
//!
//! ## Ordering
//!
//! [`RailId`] compares chronologically when compared as a binary value because
//! the timestamp is the most significant field. Do **not** assume that sorting
//! the Base64URL strings is chronological; standard Base64 does not preserve
//! numeric order across all boundaries.

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod bulk;
mod clock;
mod error;
mod id;
mod os_random;
mod rand;
mod tpm;

use std::time::{SystemTime, UNIX_EPOCH};

pub use bulk::{
    bulk_worker_count, for_each_many_encoded, for_each_many_strings, generate_many,
    generate_many_encoded_batch, generate_many_strings, generate_many_with_source,
};
pub use error::RailIdError;
pub use id::{
    decode, encode, encode_array, from_parts, random, timestamp, EncodedBatch, EncodedRailId,
    RailId, BINARY_LEN, BITS, ENCODED_LEN, RANDOM_ACQUIRE_BYTES, RANDOM_BITS, TICKS_PER_SECOND,
    TIMESTAMP_BITS, TIMESTAMP_MAX,
};
pub use rand::{RandomSource, SystemRandomSource};

/// Version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Number of bytes requested from the TPM in one bulk-generation read.
///
/// `TPM2_GetRandom` typically returns up to the TPM's maximum digest size
/// (commonly 64 bytes). Bulk generation slices these bytes into 19-byte chunks;
/// each RailID still receives 19 fresh TPM bytes.
pub const TPM_BULK_READ_BYTES: usize = 64;

/// Minimum number of IDs assigned to one parallel OS-only worker.
///
/// Small batches stay sequential because thread startup would cost more than
/// the generation work itself.
pub const BULK_MIN_IDS_PER_WORKER: usize = 4_096;

/// Size of the per-worker OS CSPRNG buffer used by bulk OS-only generation.
///
/// The buffer is allocated per worker, not globally. At the default 8 KiB it
/// holds enough random bytes for roughly 437 IDs; twelve workers therefore use
/// about 96 KiB, not an unbounded pool.
pub const OS_RANDOM_BUFFER_BYTES: usize = 8 * 1024;

/// The entropy profile used to create a RailID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// RailID v1 TPM profile: `TPM2_GetRandom` XOR OS CSPRNG, reduced to 150 bits.
    ///
    /// This is the default and the only profile that conforms to the TPM 2.0
    /// profile in the specification.
    TpmHybrid,
    /// Explicitly selected degraded profile using only the OS CSPRNG.
    ///
    /// This exists for systems where a TPM is intentionally not used. It is
    /// distinguishable from [`Profile::TpmHybrid`] at every call site and is
    /// never selected as a fallback.
    OsOnlyDegraded,
}

impl Profile {
    /// Return `true` for explicitly degraded profiles that do not use the TPM.
    #[must_use]
    pub const fn is_degraded(self) -> bool {
        matches!(self, Self::OsOnlyDegraded)
    }
}

/// Generate a RailID with the conforming TPM hybrid profile.
///
/// # Errors
///
/// Returns [`RailIdError::TpmUnavailable`] or [`RailIdError::TpmRandomFailure`]
/// when the TPM cannot provide randomness, [`RailIdError::OsRandomFailure`] when
/// the OS CSPRNG fails, or [`RailIdError::TimestampOutOfRange`] when the system
/// clock cannot be represented.
pub fn generate() -> Result<RailId, RailIdError> {
    generate_with_profile(Profile::TpmHybrid)
}

/// Generate a RailID and return its 32-character encoding.
///
/// # Errors
///
/// See [`generate`].
pub fn generate_string() -> Result<String, RailIdError> {
    generate().map(|id| id.encode())
}

/// Generate a RailID using an explicitly selected profile.
///
/// Passing [`Profile::OsOnlyDegraded`] is a deliberate opt-in to degraded
/// randomness. Prefer [`generate`] for the standard profile.
///
/// # Errors
///
/// See [`generate`].
pub fn generate_with_profile(profile: Profile) -> Result<RailId, RailIdError> {
    let ticks = clock::current_ticks()?;
    let mut source = SystemRandomSource;
    generate_with_source(profile, ticks, &mut source)
}

/// Generate a degraded RailID from the OS CSPRNG only.
///
/// This is equivalent to
/// `generate_with_profile(Profile::OsOnlyDegraded)` and is provided as an
/// explicit, self-documenting escape hatch. It is not the TPM profile.
///
/// # Errors
///
/// See [`generate`].
pub fn generate_os_only() -> Result<RailId, RailIdError> {
    generate_with_profile(Profile::OsOnlyDegraded)
}

/// Generate a RailID using a caller-supplied time.
///
/// This is useful for deterministic tests and for replaying timestamps. The
/// system entropy sources are still used.
///
/// # Errors
///
/// See [`generate`].
pub fn generate_with_profile_and_time(
    profile: Profile,
    now: SystemTime,
) -> Result<RailId, RailIdError> {
    let ticks = ticks_from_system_time(now)?;
    let mut source = SystemRandomSource;
    generate_with_source(profile, ticks, &mut source)
}

/// Generate a RailID from an explicit timestamp and entropy source.
///
/// The timestamp is in 100 ms ticks since the Unix Epoch. This entry point is
/// intended for conformance tests and custom TPM transports; it performs the
/// same checks and hybrid combination as [`generate`].
///
/// # Errors
///
/// Returns [`RailIdError::TimestampOutOfRange`] for timestamps wider than 42
/// bits, or the error reported by the requested random source.
pub fn generate_with_source<S: RandomSource + ?Sized>(
    profile: Profile,
    timestamp_ticks: u64,
    source: &mut S,
) -> Result<RailId, RailIdError> {
    if timestamp_ticks > TIMESTAMP_MAX {
        return Err(RailIdError::TimestampOutOfRange);
    }

    let random = match profile {
        Profile::TpmHybrid => {
            let mut tpm_random = [0u8; RANDOM_ACQUIRE_BYTES];
            source.fill_tpm(&mut tpm_random)?;

            let mut os_random = [0u8; RANDOM_ACQUIRE_BYTES];
            source.fill_os(&mut os_random)?;

            rand::combine_hybrid(&tpm_random, &os_random)
        }
        Profile::OsOnlyDegraded => {
            let mut os_random = [0u8; RANDOM_ACQUIRE_BYTES];
            source.fill_os(&mut os_random)?;

            rand::combine_hybrid(&[0u8; RANDOM_ACQUIRE_BYTES], &os_random)
        }
    };

    RailId::from_parts(timestamp_ticks, random)
}

/// Convert Unix milliseconds to 42-bit timestamp ticks.
///
/// # Errors
///
/// Returns [`RailIdError::TimestampOutOfRange`] when the resulting tick count
/// exceeds 42 bits.
pub fn timestamp_ticks_from_unix_millis(unix_millis: u64) -> Result<u64, RailIdError> {
    let ticks = unix_millis / 100;
    if ticks > TIMESTAMP_MAX {
        Err(RailIdError::TimestampOutOfRange)
    } else {
        Ok(ticks)
    }
}

pub(crate) fn ticks_from_system_time(now: SystemTime) -> Result<u64, RailIdError> {
    let elapsed = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RailIdError::TimestampOutOfRange)?;
    let ticks = elapsed.as_millis() / 100;
    if ticks > u128::from(TIMESTAMP_MAX) {
        Err(RailIdError::TimestampOutOfRange)
    } else {
        Ok(ticks as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn unix_millis_are_truncated_to_100ms_ticks() {
        assert_eq!(timestamp_ticks_from_unix_millis(0).unwrap(), 0);
        assert_eq!(timestamp_ticks_from_unix_millis(99).unwrap(), 0);
        assert_eq!(timestamp_ticks_from_unix_millis(100).unwrap(), 1);
        assert_eq!(timestamp_ticks_from_unix_millis(199).unwrap(), 1);
        assert_eq!(timestamp_ticks_from_unix_millis(200).unwrap(), 2);
    }

    #[test]
    fn timestamps_beyond_42_bits_are_rejected() {
        let max_millis = TIMESTAMP_MAX * 100;
        assert_eq!(
            timestamp_ticks_from_unix_millis(max_millis).unwrap(),
            TIMESTAMP_MAX
        );
        assert_eq!(
            timestamp_ticks_from_unix_millis(max_millis + 100),
            Err(RailIdError::TimestampOutOfRange)
        );
    }

    #[test]
    fn system_time_before_the_epoch_is_rejected() {
        let before = UNIX_EPOCH - Duration::from_secs(1);
        assert_eq!(
            generate_with_profile_and_time(Profile::TpmHybrid, before),
            Err(RailIdError::TimestampOutOfRange)
        );
    }
}

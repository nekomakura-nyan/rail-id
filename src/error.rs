//! Errors returned by RailID operations.

use core::fmt;

/// Errors that can occur while generating or decoding a [`crate::RailId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailIdError {
    /// The timestamp does not fit in the 42-bit RailID v1 field.
    TimestampOutOfRange,
    /// No usable TPM 2.0 device or TSS transport was available.
    TpmUnavailable,
    /// The TPM was reached but did not provide random bytes.
    TpmRandomFailure,
    /// The operating system CSPRNG failed.
    OsRandomFailure,
    /// The input is not valid unpadded Base64URL.
    InvalidEncoding,
    /// The input is not exactly 32 characters long.
    InvalidLength,
    /// A requested batch size cannot be represented in memory.
    CapacityOverflow,
}

impl fmt::Display for RailIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TimestampOutOfRange => "timestamp does not fit in 42 bits",
            Self::TpmUnavailable => "TPM 2.0 is unavailable",
            Self::TpmRandomFailure => "TPM 2.0 random generation failed",
            Self::OsRandomFailure => "operating system CSPRNG failed",
            Self::InvalidEncoding => "invalid unpadded Base64URL encoding",
            Self::InvalidLength => "RailID must be exactly 32 characters",
            Self::CapacityOverflow => "requested ID batch is too large",
        };
        f.write_str(message)
    }
}

impl std::error::Error for RailIdError {}

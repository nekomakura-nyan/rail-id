//! Thin feature gate over the optional `railid-tpm` backend.
//!
//! The core crate has no system TPM dependency. Without the `tpm` feature,
//! every call fails with [`crate::RailIdError::TpmUnavailable`]; there is no
//! silent fallback to the OS CSPRNG.

use crate::RailIdError;

pub(crate) fn fill(out: &mut [u8]) -> Result<(), RailIdError> {
    imp::fill(out)
}

#[cfg(feature = "tpm")]
mod imp {
    use railid_tpm::TpmError;

    use crate::RailIdError;

    pub(crate) fn fill(out: &mut [u8]) -> Result<(), RailIdError> {
        railid_tpm::fill(out).map_err(|error| match error {
            TpmError::Unavailable => RailIdError::TpmUnavailable,
            TpmError::RandomFailure => RailIdError::TpmRandomFailure,
            // `TpmError` is non-exhaustive; fail closed on future variants.
            _ => RailIdError::TpmRandomFailure,
        })
    }
}

#[cfg(not(feature = "tpm"))]
mod imp {
    use crate::RailIdError;

    pub(crate) fn fill(_out: &mut [u8]) -> Result<(), RailIdError> {
        Err(RailIdError::TpmUnavailable)
    }
}

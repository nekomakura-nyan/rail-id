//! TPM 2.0 `GetRandom` backend for RailID.
//!
//! * **Linux:** the TSS Enhanced System API through `tss-esapi` (`linux` feature).
//! * **Windows:** the TPM Base Services API (`Tbs.dll`) (`windows` feature).
//!
//! On other targets, or when the platform feature is disabled, every call
//! fails with [`TpmError::Unavailable`]; there is no silent fallback to the OS
//! CSPRNG.
//!
//! This crate is normally used through the `tpm` feature of the `railid` crate.
//! It is a separate crate so the core `railid` build has no system library
//! dependency.

use core::fmt;

/// Failure from the TPM backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TpmError {
    /// The TPM device, TBS service, or TSS transport is unavailable.
    Unavailable,
    /// The TPM was reached but returned an unusable response.
    RandomFailure,
}

impl fmt::Display for TpmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("TPM 2.0 is unavailable"),
            Self::RandomFailure => formatter.write_str("TPM random generation failed"),
        }
    }
}

impl std::error::Error for TpmError {}

/// Fill `out` from `TPM2_GetRandom`.
///
/// # Errors
///
/// Returns [`TpmError::Unavailable`] when the platform backend is not compiled
/// or the device cannot be opened, and [`TpmError::RandomFailure`] for invalid
/// or failed TPM responses.
pub fn fill(out: &mut [u8]) -> Result<(), TpmError> {
    imp::fill(out)
}

/// Fill a buffer from a callback that may return fewer bytes than requested.
///
/// Empty responses are retried; a run of consecutive empty responses that
/// never makes progress, or an oversized response, is a random failure.
#[cfg_attr(
    not(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    )),
    allow(dead_code)
)]
fn fill_from<F>(out: &mut [u8], mut request: F) -> Result<(), TpmError>
where
    F: FnMut(&mut [u8]) -> Result<usize, TpmError>,
{
    const MAX_EMPTY_RESPONSES: usize = 64;

    let mut filled = 0usize;
    let mut empty_responses = 0usize;

    while filled < out.len() {
        let received = request(&mut out[filled..])?;
        let remaining = out.len() - filled;
        if received > remaining {
            return Err(TpmError::RandomFailure);
        }
        if received == 0 {
            empty_responses += 1;
            if empty_responses >= MAX_EMPTY_RESPONSES {
                return Err(TpmError::RandomFailure);
            }
        } else {
            empty_responses = 0;
        }
        filled += received;
    }

    Ok(())
}

#[cfg(all(feature = "linux", target_os = "linux"))]
mod imp {
    use std::str::FromStr;
    use std::sync::Mutex;

    use tss_esapi::tcti_ldr::{DeviceConfig, TctiNameConf};
    use tss_esapi::Context;

    use crate::fill_from;
    use crate::TpmError;

    /// A TPM context is relatively expensive to open, so successful contexts
    /// are reused. A failed open is not cached: a TPM that appears later can
    /// still be used without restarting the process.
    static CONTEXT: Mutex<Option<Context>> = Mutex::new(None);

    /// Common Linux TPM device nodes. The kernel resource manager
    /// (`/dev/tpmrm0`) is preferred because it multiplexes access between
    /// processes.
    const DEVICE_CANDIDATES: [&str; 2] = ["/dev/tpmrm0", "/dev/tpm0"];

    pub(crate) fn fill(out: &mut [u8]) -> Result<(), TpmError> {
        if out.is_empty() {
            return Ok(());
        }

        let mut guard = CONTEXT
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_none() {
            *guard = Some(open_context()?);
        }
        let context = guard.as_mut().ok_or(TpmError::Unavailable)?;

        let result = fill_from(out, |buffer| {
            if buffer.is_empty() {
                return Ok(0);
            }

            let requested = buffer.len().min(64);
            let mut digest = None;
            for attempt in 0..3 {
                match context.get_random(requested) {
                    Ok(value) => {
                        digest = Some(value);
                        break;
                    }
                    Err(_) if attempt < 2 => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(_) => return Err(TpmError::RandomFailure),
                }
            }
            let digest = digest.ok_or(TpmError::RandomFailure)?;
            let bytes = digest.value();

            if bytes.len() > buffer.len() {
                return Err(TpmError::RandomFailure);
            }

            buffer[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        });

        if result.is_err() {
            // Drop a possibly stale TSS context so a later call can reopen it.
            *guard = None;
        }
        result
    }

    fn open_context() -> Result<Context, TpmError> {
        // Honour TPM2TOOLS_TCTI / TCTI / TEST_TCTI when configured. If one is
        // set but invalid, fail rather than silently ignoring the operator's
        // transport choice.
        const TCTI_ENV_VARS: [&str; 3] = ["TPM2TOOLS_TCTI", "TCTI", "TEST_TCTI"];
        if TCTI_ENV_VARS
            .iter()
            .any(|name| std::env::var_os(name).is_some())
        {
            let tcti =
                TctiNameConf::from_environment_variable().map_err(|_| TpmError::Unavailable)?;
            return Context::new(tcti).map_err(|_| TpmError::Unavailable);
        }

        for candidate in DEVICE_CANDIDATES {
            let config = DeviceConfig::from_str(candidate).map_err(|_| TpmError::Unavailable)?;
            if let Ok(context) = Context::new(TctiNameConf::Device(config)) {
                return Ok(context);
            }
        }

        Err(TpmError::Unavailable)
    }
}

#[cfg(all(feature = "windows", target_os = "windows"))]
#[allow(unsafe_code)]
mod imp {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::Mutex;

    use windows_sys::Win32::System::TpmBaseServices::{
        Tbsi_Context_Create, Tbsip_Context_Close, Tbsip_Submit_Command, TBS_COMMAND_LOCALITY_ZERO,
        TBS_COMMAND_PRIORITY_NORMAL, TBS_CONTEXT_PARAMS, TBS_CONTEXT_PARAMS2,
        TBS_CONTEXT_PARAMS2_0, TBS_CONTEXT_VERSION_TWO, TBS_SUCCESS,
    };

    use crate::fill_from;
    use crate::TpmError;

    /// TPM 2.0 command header is 10 bytes: tag + commandSize + commandCode.
    const COMMAND_HEADER_SIZE: usize = 10;
    /// TPM 2.0 response header is 10 bytes: tag + responseSize + responseCode.
    const RESPONSE_HEADER_SIZE: usize = 10;
    /// `TPM2B_DIGEST` starts with a 2-byte size field.
    const TPM2B_SIZE_FIELD: usize = 2;
    const TPM_ST_NO_SESSIONS: u16 = 0x8001;
    const TPM_CC_GET_RANDOM: u32 = 0x0000_017b;
    const TPM_RC_SUCCESS: u32 = 0;
    const TPM_RC_RETRY: u32 = 0x0908;
    const TPM_RC_YIELDED: u32 = 0x0909;
    const TPM_RC_TESTING: u32 = 0x090a;
    /// 10-byte header + 2-byte digest size + up to 64 random bytes.
    const MAX_RESPONSE_SIZE: usize = 128;

    const TBS_E_INVALID_CONTEXT: u32 = 0x8028_4004;
    const TBS_E_IOERROR: u32 = 0x8028_4006;
    const TBS_E_SERVICE_NOT_RUNNING: u32 = 0x8028_4008;
    const TBS_E_TOO_MANY_TBS_CONTEXTS: u32 = 0x8028_4009;
    const TBS_E_TOO_MANY_RESOURCES: u32 = 0x8028_400a;
    const TBS_E_SERVICE_START_PENDING: u32 = 0x8028_400b;
    const TBS_E_COMMAND_CANCELED: u32 = 0x8028_400d;
    const TBS_E_TPM_NOT_FOUND: u32 = 0x8028_400f;
    const TBS_E_SERVICE_DISABLED: u32 = 0x8028_4010;
    const TBS_E_ACCESS_DENIED: u32 = 0x8028_4012;

    fn transient_tbs_status(status: u32) -> bool {
        matches!(
            status,
            TBS_E_IOERROR
                | TBS_E_SERVICE_START_PENDING
                | TBS_E_TOO_MANY_TBS_CONTEXTS
                | TBS_E_TOO_MANY_RESOURCES
                | TBS_E_COMMAND_CANCELED
        )
    }

    fn unavailable_tbs_status(status: u32) -> bool {
        matches!(
            status,
            TBS_E_INVALID_CONTEXT
                | TBS_E_SERVICE_NOT_RUNNING
                | TBS_E_TPM_NOT_FOUND
                | TBS_E_SERVICE_DISABLED
                | TBS_E_ACCESS_DENIED
        )
    }

    fn transient_tpm_rc(response_code: u32) -> bool {
        matches!(
            response_code,
            TPM_RC_RETRY | TPM_RC_YIELDED | TPM_RC_TESTING
        )
    }

    thread_local! {
        /// TBS contexts are per-thread handles; keeping one per thread avoids
        /// reopening while remaining `Send`-free and safe to use in tests.
        static CONTEXT: RefCell<Option<TbsContext>> = const { RefCell::new(None) };
    }

    /// A TPM is a single device. TBS allows multiple contexts, but concurrent
    /// submissions can fail transiently; serialize all commands in this
    /// process so multi-threaded generation remains reliable.
    static TPM_LOCK: Mutex<()> = Mutex::new(());

    struct TbsContext(*mut c_void);

    impl TbsContext {
        fn open() -> Result<Self, TpmError> {
            let params = TBS_CONTEXT_PARAMS2 {
                version: TBS_CONTEXT_VERSION_TWO,
                // includeTpm20 is bit 2 of the TBS_CONTEXT_PARAMS2 flag word.
                Anonymous: TBS_CONTEXT_PARAMS2_0 { asUINT32: 1 << 2 },
            };
            let mut handle = ptr::null_mut();

            // SAFETY: `params` is a valid TBS_CONTEXT_PARAMS2. TBS reads only
            // the common version field (and the flags for version 2), which is
            // why the pointer is cast to TBS_CONTEXT_PARAMS. `handle` is a
            // valid out-pointer for the created context.
            let status = unsafe {
                Tbsi_Context_Create(
                    &params as *const TBS_CONTEXT_PARAMS2 as *const TBS_CONTEXT_PARAMS,
                    &mut handle,
                )
            };

            if status != TBS_SUCCESS || handle.is_null() {
                // TBS should not return a handle on failure, but close a
                // defensive non-null handle instead of leaking it.
                if !handle.is_null() {
                    // SAFETY: handle was returned by Tbsi_Context_Create.
                    unsafe {
                        Tbsip_Context_Close(handle);
                    }
                }
                return Err(TpmError::Unavailable);
            }

            Ok(Self(handle))
        }

        fn get_random(&self, requested: u16, out: &mut [u8]) -> Result<usize, TpmError> {
            let requested = requested.min(out.len() as u16).min(64);
            if requested == 0 {
                return Ok(0);
            }
            let command = get_random_command(requested);

            for attempt in 0..4u32 {
                let mut response = [0u8; MAX_RESPONSE_SIZE];
                let mut response_len = response.len() as u32;

                // SAFETY: `self.0` is a live context created by
                // Tbsi_Context_Create. `command` and `response` are valid for
                // their stated lengths, and TBS writes at most `response_len`
                // bytes to `response`.
                let status = unsafe {
                    Tbsip_Submit_Command(
                        self.0,
                        TBS_COMMAND_LOCALITY_ZERO,
                        TBS_COMMAND_PRIORITY_NORMAL,
                        command.as_ptr(),
                        command.len() as u32,
                        response.as_mut_ptr(),
                        &mut response_len,
                    )
                };

                if status != TBS_SUCCESS {
                    if transient_tbs_status(status) && attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(2 << attempt));
                        continue;
                    }
                    return Err(if unavailable_tbs_status(status) {
                        TpmError::Unavailable
                    } else {
                        TpmError::RandomFailure
                    });
                }

                let response_len = response_len as usize;
                if response_len > response.len() || response_len < RESPONSE_HEADER_SIZE {
                    return Err(TpmError::RandomFailure);
                }

                let response_code =
                    u32::from_be_bytes([response[6], response[7], response[8], response[9]]);
                if response_code != TPM_RC_SUCCESS {
                    if transient_tpm_rc(response_code) && attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(2 << attempt));
                        continue;
                    }
                    return Err(TpmError::RandomFailure);
                }

                let bytes = parse_get_random_response(&response[..response_len])?;
                if bytes.len() > out.len() {
                    return Err(TpmError::RandomFailure);
                }

                out[..bytes.len()].copy_from_slice(bytes);
                return Ok(bytes.len());
            }

            Err(TpmError::RandomFailure)
        }
    }

    impl Drop for TbsContext {
        fn drop(&mut self) {
            // SAFETY: `self.0` was created by Tbsi_Context_Create and is closed
            // exactly once here.
            unsafe {
                Tbsip_Context_Close(self.0);
            }
        }
    }

    pub(crate) fn fill(out: &mut [u8]) -> Result<(), TpmError> {
        if out.is_empty() {
            return Ok(());
        }

        // The lock only serializes TPM access; a poisoned lock carries no
        // invalid state, so recover it instead of disabling the backend.
        let _tpm_guard = TPM_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        CONTEXT.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_none() {
                *slot = Some(TbsContext::open()?);
            }

            let result = {
                let context = slot.as_ref().ok_or(TpmError::Unavailable)?;
                fill_from(out, |buffer| {
                    if buffer.is_empty() {
                        return Ok(0);
                    }

                    let requested =
                        u16::try_from(buffer.len()).map_err(|_| TpmError::RandomFailure)?;
                    context.get_random(requested, buffer)
                })
            };

            if result.is_err() {
                // Drop a possibly stale TBS handle so the next call reopens it.
                *slot = None;
            }

            result
        })
    }

    fn get_random_command(requested: u16) -> [u8; COMMAND_HEADER_SIZE + TPM2B_SIZE_FIELD] {
        let command_size = (COMMAND_HEADER_SIZE + TPM2B_SIZE_FIELD) as u32;
        let mut command = [0u8; COMMAND_HEADER_SIZE + TPM2B_SIZE_FIELD];

        command[0..2].copy_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        command[2..6].copy_from_slice(&command_size.to_be_bytes());
        command[6..10].copy_from_slice(&TPM_CC_GET_RANDOM.to_be_bytes());
        command[10..12].copy_from_slice(&requested.to_be_bytes());

        command
    }

    fn parse_get_random_response(response: &[u8]) -> Result<&[u8], TpmError> {
        const MIN_RESPONSE_SIZE: usize = RESPONSE_HEADER_SIZE + TPM2B_SIZE_FIELD;

        if response.len() < MIN_RESPONSE_SIZE {
            return Err(TpmError::RandomFailure);
        }

        let tag = u16::from_be_bytes([response[0], response[1]]);
        if tag != TPM_ST_NO_SESSIONS {
            return Err(TpmError::RandomFailure);
        }

        let declared_size =
            u32::from_be_bytes([response[2], response[3], response[4], response[5]]) as usize;
        if declared_size != response.len() {
            return Err(TpmError::RandomFailure);
        }

        let response_code =
            u32::from_be_bytes([response[6], response[7], response[8], response[9]]);
        if response_code != TPM_RC_SUCCESS {
            return Err(TpmError::RandomFailure);
        }

        let random_len = u16::from_be_bytes([response[10], response[11]]) as usize;
        if random_len != response.len() - MIN_RESPONSE_SIZE {
            return Err(TpmError::RandomFailure);
        }

        Ok(&response[MIN_RESPONSE_SIZE..])
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn success_response(bytes: &[u8]) -> Vec<u8> {
            let mut response = Vec::new();
            response.extend_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
            response.extend_from_slice(
                &((RESPONSE_HEADER_SIZE + TPM2B_SIZE_FIELD + bytes.len()) as u32).to_be_bytes(),
            );
            response.extend_from_slice(&TPM_RC_SUCCESS.to_be_bytes());
            response.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            response.extend_from_slice(bytes);
            response
        }

        #[test]
        fn get_random_command_has_the_tpm_wire_layout() {
            assert_eq!(
                get_random_command(19),
                [0x80, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x01, 0x7b, 0x00, 0x13]
            );
        }

        #[test]
        fn parses_a_successful_response() {
            let response = success_response(&[1, 2, 3]);
            assert_eq!(parse_get_random_response(&response).unwrap(), &[1, 2, 3]);
        }

        #[test]
        fn parses_a_partial_response() {
            let response = success_response(&[0xaa]);
            assert_eq!(parse_get_random_response(&response).unwrap(), &[0xaa]);
        }

        #[test]
        fn rejects_tpm_error_response() {
            let mut response = success_response(&[]);
            response[9] = 0x01;
            assert_eq!(
                parse_get_random_response(&response),
                Err(TpmError::RandomFailure)
            );
        }

        #[test]
        fn rejects_truncated_response() {
            let response = success_response(&[1, 2, 3]);
            assert_eq!(
                parse_get_random_response(&response[..response.len() - 1]),
                Err(TpmError::RandomFailure)
            );
        }

        #[test]
        fn rejects_response_size_mismatch() {
            let mut response = success_response(&[1, 2, 3]);
            response[5] += 1;
            assert_eq!(
                parse_get_random_response(&response),
                Err(TpmError::RandomFailure)
            );
        }

        #[test]
        fn rejects_digest_size_mismatch() {
            let mut response = success_response(&[1, 2, 3]);
            response[11] += 1;
            assert_eq!(
                parse_get_random_response(&response),
                Err(TpmError::RandomFailure)
            );
        }
    }
}

#[cfg(not(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
)))]
mod imp {
    use crate::TpmError;

    pub(crate) fn fill(_out: &mut [u8]) -> Result<(), TpmError> {
        Err(TpmError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_from_accepts_partial_responses() {
        let mut out = [0u8; 19];
        let mut calls = 0usize;
        fill_from(&mut out, |buffer| {
            calls += 1;
            let count = buffer.len().min(7);
            buffer[..count].fill(calls as u8);
            Ok(count)
        })
        .expect("partial responses should fill the buffer");
        assert_eq!(calls, 3);
        assert_eq!(out[..7], [1u8; 7]);
        assert_eq!(out[7..14], [2u8; 7]);
        assert_eq!(out[14..], [3u8; 5]);
    }

    #[test]
    fn fill_from_retries_empty_responses() {
        let mut out = [0u8; 19];
        let mut calls = 0usize;
        fill_from(&mut out, |buffer| {
            calls += 1;
            if calls <= 2 {
                return Ok(0);
            }
            buffer.fill(0xaa);
            Ok(buffer.len())
        })
        .expect("empty response should be retried");
        assert_eq!(calls, 3);
        assert_eq!(out, [0xaa; 19]);
    }

    #[test]
    fn fill_from_bounds_repeated_empty_responses() {
        let mut out = [0u8; 19];
        let mut calls = 0usize;
        let error = fill_from(&mut out, |_| {
            calls += 1;
            Ok(0)
        })
        .unwrap_err();
        assert_eq!(error, TpmError::RandomFailure);
        assert_eq!(calls, 64);
    }

    #[test]
    fn fill_from_rejects_oversized_response() {
        let mut out = [0u8; 19];
        let error = fill_from(&mut out, |buffer| Ok(buffer.len() + 1)).unwrap_err();
        assert_eq!(error, TpmError::RandomFailure);
    }
}

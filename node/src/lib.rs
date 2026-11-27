//! N-API bindings for RailID.
//!
//! The addon exposes synchronous single-ID/batch APIs and asynchronous batch
//! APIs. Bulk work runs on the libuv thread pool through `AsyncTask`, while
//! the TPM backend remains serialized by its process-wide mutex.

use napi::bindgen_prelude::{AsyncTask, Buffer, BufferSlice, Result, ToNapiValue, TypeName};
use napi::{sys, Env, Task, ValueType};
use napi_derive::napi;
use std::ptr;

use railid::{Profile, RailId, RailIdError, ENCODED_LEN};

/// Hard cap for encoded batches returned through N-API (32 MiB at 1M IDs).
const MAX_ENCODED_BATCH_IDS: u32 = 1_000_000;
/// Hard cap for owned JS String arrays (roughly 10-25 MiB at 100k IDs).
const MAX_STRING_BATCH_IDS: u32 = 100_000;
/// TPM calls are synchronous and serialized, so keep Node-side TPM batches short.
const MAX_TPM_BATCH_IDS: u32 = 1_024;

fn batch_limit(os_only: Option<bool>, maximum: u32) -> u32 {
    if os_only.unwrap_or(false) {
        maximum
    } else {
        maximum.min(MAX_TPM_BATCH_IDS)
    }
}

fn check_count(count: u32, maximum: u32) -> Result<usize> {
    if count > maximum {
        return Err(napi::Error::from_reason(format!(
            "count {count} exceeds binding limit {maximum}"
        )));
    }
    Ok(count as usize)
}

fn to_napi_error(error: RailIdError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

fn profile(os_only: Option<bool>) -> Profile {
    if os_only.unwrap_or(false) {
        Profile::OsOnlyDegraded
    } else {
        Profile::TpmHybrid
    }
}

/// A 32-byte ASCII RailID encoded as a JavaScript Latin-1 string.
///
/// Encoding directly from the stack avoids a Rust `String` allocation and an
/// extra UTF-8 validation pass in the N-API string conversion.
pub struct Latin1RailId([u8; ENCODED_LEN]);

impl Latin1RailId {
    fn from_id(id: RailId) -> Self {
        let encoded = id.encode_array();
        Self(*encoded.as_bytes())
    }
}

impl TypeName for Latin1RailId {
    fn type_name() -> &'static str {
        "String"
    }

    fn value_type() -> ValueType {
        ValueType::String
    }
}

impl ToNapiValue for Latin1RailId {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> Result<sys::napi_value> {
        let mut result = ptr::null_mut();
        napi::check_status!(
            unsafe {
                sys::napi_create_string_latin1(
                    env,
                    value.0.as_ptr().cast(),
                    value.0.len() as isize,
                    &mut result,
                )
            },
            "Failed to create a Latin-1 RailID string"
        )?;
        Ok(result)
    }
}

/// Generate one RailID with the TPM hybrid profile.
#[napi]
pub fn generate() -> Result<Latin1RailId> {
    railid::generate()
        .map(Latin1RailId::from_id)
        .map_err(to_napi_error)
}

/// Generate one RailID with the explicitly degraded OS-only profile.
#[napi]
pub fn generate_os_only() -> Result<Latin1RailId> {
    railid::generate_os_only()
        .map(Latin1RailId::from_id)
        .map_err(to_napi_error)
}

/// Generate one RailID with the TPM hybrid profile and return its 32 ASCII
/// bytes as a Buffer, without creating a JavaScript string.
#[napi]
pub fn generate_buffer(env: Env) -> Result<Buffer> {
    railid::generate()
        .map_err(to_napi_error)
        .and_then(|id| encoded_buffer(&env, id))
}

/// Generate one RailID with the OS-only profile and return its 32 ASCII bytes
/// as a Buffer, without creating a JavaScript string.
#[napi]
pub fn generate_os_only_buffer(env: Env) -> Result<Buffer> {
    railid::generate_os_only()
        .map_err(to_napi_error)
        .and_then(|id| encoded_buffer(&env, id))
}

fn encoded_buffer(env: &Env, id: RailId) -> Result<Buffer> {
    let encoded = id.encode_array();
    BufferSlice::copy_from(env, encoded.as_bytes())?.into_buffer(env)
}

/// Generate `count` IDs as a JavaScript string array.
#[napi]
pub fn generate_many(count: u32, os_only: Option<bool>) -> Result<Vec<Latin1RailId>> {
    let count = check_count(count, batch_limit(os_only, MAX_STRING_BATCH_IDS))?;
    railid::generate_many(count, profile(os_only))
        .map(|ids| ids.into_iter().map(Latin1RailId::from_id).collect())
        .map_err(to_napi_error)
}

/// Generate `count` IDs as one contiguous Base64URL Buffer (`count * 32` bytes).
#[napi]
pub fn generate_many_encoded(count: u32, os_only: Option<bool>) -> Result<Buffer> {
    let count = check_count(count, batch_limit(os_only, MAX_ENCODED_BATCH_IDS))?;
    railid::generate_many_encoded_batch(count, profile(os_only))
        .map(|batch| Buffer::from(batch.into_bytes()))
        .map_err(to_napi_error)
}

/// Generate `count` IDs asynchronously as a JavaScript string array.
#[napi]
pub fn generate_many_async(
    count: u32,
    os_only: Option<bool>,
) -> Result<AsyncTask<GenerateManyTask>> {
    let count = check_count(count, batch_limit(os_only, MAX_STRING_BATCH_IDS))?;
    Ok(AsyncTask::new(GenerateManyTask {
        count,
        profile: profile(os_only),
    }))
}

/// Generate `count` IDs asynchronously as one contiguous Base64URL Buffer.
#[napi]
pub fn generate_many_encoded_async(
    count: u32,
    os_only: Option<bool>,
) -> Result<AsyncTask<GenerateManyEncodedTask>> {
    let count = check_count(count, batch_limit(os_only, MAX_ENCODED_BATCH_IDS))?;
    Ok(AsyncTask::new(GenerateManyEncodedTask {
        count,
        profile: profile(os_only),
    }))
}

/// Decoded RailID fields.
#[napi(object)]
pub struct DecodedRailId {
    /// Timestamp as Unix milliseconds. Safe as a JavaScript number until year 15907.
    pub timestamp_ms: f64,
    /// 150-bit random field as 38 lowercase hex characters.
    pub random_hex: String,
    /// Canonical 32-character Base64URL encoding.
    pub encoded: String,
}

/// Decode and validate a RailID.
#[napi]
pub fn decode(input: String) -> Result<DecodedRailId> {
    let id = RailId::decode(&input).map_err(to_napi_error)?;
    Ok(DecodedRailId {
        timestamp_ms: id.timestamp_millis() as f64,
        random_hex: hex(&id.random()),
        encoded: id.encode(),
    })
}

/// RailID crate version.
#[napi]
pub fn version() -> String {
    railid::VERSION.to_owned()
}

/// Native backend compiled into this addon: `"tpm"` or `"os-only"`.
#[napi]
pub fn backend() -> String {
    if cfg!(feature = "tpm") {
        "tpm".to_owned()
    } else {
        "os-only".to_owned()
    }
}

pub struct GenerateManyTask {
    count: usize,
    profile: Profile,
}

impl Task for GenerateManyTask {
    type Output = Vec<Latin1RailId>;
    type JsValue = Vec<Latin1RailId>;

    fn compute(&mut self) -> Result<Self::Output> {
        railid::generate_many(self.count, self.profile)
            .map(|ids| ids.into_iter().map(Latin1RailId::from_id).collect())
            .map_err(to_napi_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct GenerateManyEncodedTask {
    count: usize,
    profile: Profile,
}

impl Task for GenerateManyEncodedTask {
    type Output = Vec<u8>;
    type JsValue = Buffer;

    fn compute(&mut self) -> Result<Self::Output> {
        railid::generate_many_encoded_batch(self.count, self.profile)
            .map(|batch| batch.into_bytes())
            .map_err(to_napi_error)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(Buffer::from(output))
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

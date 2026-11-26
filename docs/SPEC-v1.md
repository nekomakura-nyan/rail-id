# RailID Specification

**Version:** 1.0.0-draft  
**Status:** Proposed  
**Implementation Language:** Rust  
**Target Environment:** Systems equipped with TPM 2.0  
**Encoding:** Base64URL (RFC 4648, Section 5), without padding

---

## 1. Abstract

RailID is a compact, time-aware, globally unique identifier designed for distributed systems, databases, applications, and security-sensitive environments.

RailID combines a timestamp with cryptographically secure random data. Its fixed-length representation is URL-safe, compact, and does not require a centralized ID allocation service.

RailID is designed to provide:

- Fixed-length identifiers
- URL-safe representation
- Time information embedded in the identifier
- High collision resistance
- Cryptographically secure randomness
- Efficient generation
- Timestamp extraction
- Rust implementation
- TPM 2.0 hardware RNG support

---

## 2. Identifier Format

RailID consists of a 192-bit binary value.

| Field | Size | Description |
|---|---:|---|
| Timestamp | 42 bits | Unix timestamp in 100ms units |
| Random | 150 bits | Cryptographically secure random value |
| **Total** | **192 bits** | **24 bytes** |

### 2.1 Binary Layout

The binary layout is MSB-first (most significant bit first).

```text
┌──────────────────────────┬─────────────────────────────────────────┐
│       Timestamp         │                 Random                  │
│        42 bits          │                150 bits                 │
└──────────────────────────┴─────────────────────────────────────────┘
                         192 bits
```

The Timestamp field is placed first, followed by the Random field.

The field boundaries are bit-aligned, not byte-aligned.

### 2.2 Encoded Format

RailID is encoded using Base64URL without padding.

| Property | Value |
|---|---|
| Binary size | 192 bits |
| Binary size | 24 bytes |
| Encoding | Base64URL |
| Padding | None |
| Encoded length | 32 characters |
| Character set | `A-Z`, `a-z`, `0-9`, `-`, `_` |

A valid RailID MUST contain exactly 32 Base64URL characters.

---

## 3. Timestamp

### 3.1 Epoch

RailID uses the Unix Epoch:

```text
1970-01-01T00:00:00Z
```

### 3.2 Precision

Timestamp precision is 100 milliseconds (0.1 seconds).

The Timestamp field represents the number of 100ms intervals elapsed since the Unix Epoch.

```text
Timestamp = floor(UnixTimeMilliseconds / 100)
```

### 3.3 Range

The Timestamp is an unsigned 42-bit integer.

```text
Minimum: 0
Maximum: 2^42 - 1
```

The representable range is approximately 13,937 years from the Unix Epoch, ending around year 15,907 CE.

### 3.4 Timestamp Requirements

Implementations MUST:

- Use UTC-based Unix time.
- Convert time to 100ms intervals.
- Store the Timestamp as an unsigned integer.
- Ensure the value fits in 42 bits.
- Reject timestamps outside the representable range.

Implementations MUST NOT use local timezone time.

---

## 4. Random Field

### 4.1 Size

The Random field MUST contain exactly 150 bits.

### 4.2 Randomness Requirements

The Random field MUST be generated using a cryptographically secure random number generator (CSPRNG).

Implementations MUST NOT use:

- `Math.random()`
- Non-cryptographic PRNGs
- Predictable seeds
- Timestamps as random seeds
- Process IDs as random seeds
- Sequential counters as a replacement for random bits

### 4.3 TPM 2.0 Profile

RailID's TPM profile uses the TPM 2.0 hardware RNG through the `TPM2_GetRandom` command.

The TPM RNG MUST be used as a source of cryptographically secure random data.

The TPM may return fewer bytes than requested. Implementations MUST handle partial responses by requesting additional bytes until the required amount is obtained or an error occurs.

### 4.4 Hybrid Randomness

The TPM profile SHOULD combine randomness from both:

1. TPM 2.0 RNG
2. Operating system CSPRNG

The two sources MUST be independently obtained.

The combination method is XOR:

```text
HybridRandom = TPMRandom XOR OSRandom
```

The resulting value MUST be reduced to 150 bits.

This provides defense in depth: assuming the sources are independent, the combined result remains unpredictable if at least one source is secure.

If either source fails, the TPM profile MUST fail ID generation rather than silently downgrade.

### 4.5 Randomness Acquisition

The implementation obtains 19 bytes (152 bits) from each source.

```text
TPMRandom = 19 bytes
OSRandom  = 19 bytes

HybridRandom = TPMRandom XOR OSRandom

Random = HybridRandom with 2 excess bits removed
```

The upper 150 bits are retained.

The two excess bits MUST be discarded without introducing bias.

---

## 5. TPM 2.0 Requirements

### 5.1 TPM Support

The TPM profile requires a functioning TPM 2.0 device.

The implementation SHOULD use the system TPM stack where available instead of implementing the TPM command protocol manually.

### 5.2 TPM RNG

The TPM RNG MUST be accessed using the TPM 2.0 `GetRandom` command.

The implementation MUST:

- Request random data from the TPM.
- Handle partial output.
- Handle TPM errors.
- Avoid assuming that the TPM always returns the full requested size.
- Avoid treating TPM errors as valid random data.

### 5.3 TPM Initialization

TPM initialization and access MUST be handled by the TPM system stack or a compatible TPM transport implementation.

The application MUST NOT attempt to initialize, clear, or alter TPM ownership as part of normal RailID generation.

### 5.4 TPM Failure

When TPM access fails, the TPM profile MUST return an error.

Silent fallback to OS-only randomness MUST NOT occur.

An implementation MAY provide a separately configured degraded mode, but degraded mode MUST be explicitly enabled and MUST be distinguishable from the standard TPM profile.

---

## 6. Generation Algorithm

### 6.1 Overview

```text
1. Obtain current Unix time in milliseconds.
2. Convert to 100ms Timestamp ticks.
3. Validate Timestamp fits within 42 bits.
4. Obtain 19 bytes from TPM RNG.
5. Obtain 19 bytes from OS CSPRNG.
6. XOR the two random buffers.
7. Remove the two excess bits to obtain 150 bits.
8. Concatenate Timestamp (42 bits) and Random (150 bits).
9. Convert the resulting 192 bits into 24 bytes.
10. Encode using unpadded Base64URL.
11. Return the 32-character RailID.
```

### 6.2 Timestamp Calculation

```text
Timestamp = floor(UnixTimeMilliseconds / 100)
```

### 6.3 Random Generation

```text
TPMRandom = TPM2_GetRandom(19 bytes)
OSRandom  = OS_CSPRNG(19 bytes)

HybridRandom = TPMRandom XOR OSRandom

Random = HybridRandom[150 most significant bits]
```

If TPM output is partial, the implementation MUST continue requesting bytes until 19 bytes are obtained.

### 6.4 Binary Assembly

The raw identifier MUST be assembled as:

```text
RawID = Timestamp[42 bits] || Random[150 bits]
```

The result MUST be exactly 192 bits (24 bytes).

The Timestamp occupies the first 42 bits, and the Random field occupies the remaining 150 bits.

### 6.5 Encoding

The 24-byte raw identifier MUST be encoded using unpadded Base64URL.

The resulting string MUST be exactly 32 characters long.

---

## 7. Decoding

RailID MUST support decoding the identifier into its binary representation.

### 7.1 Validation

A decoder MUST validate:

1. The input contains exactly 32 characters.
2. Every character belongs to the Base64URL alphabet.
3. Decoding produces exactly 24 bytes.
4. The encoded representation is canonical.
5. No invalid padding is present.

### 7.2 Timestamp Extraction

The Timestamp is the upper 42 bits of the 192-bit binary value.

```text
Timestamp = RawID >> 150
```

The corresponding Unix time in milliseconds is:

```text
UnixTimeMilliseconds = Timestamp × 100
```

### 7.3 Random Extraction

The Random field is the lower 150 bits of the binary value.

```text
Random = RawID AND (2^150 - 1)
```

---

## 8. Ordering

RailID places the Timestamp field before the Random field.

This means the underlying binary representation is timestamp-first.

However, standard Base64URL encoding does not guarantee that the lexicographical ordering of encoded strings matches chronological ordering.

Therefore:

- Implementations MAY sort decoded binary values to obtain timestamp-first ordering.
- Implementations MUST NOT assume standard Base64URL string sorting is chronological.
- Chronological ordering is guaranteed only by comparing the Timestamp field.
- IDs generated within the same 100ms interval have no guaranteed order.

A future RailID encoding profile MAY define a custom ordered alphabet if lexicographical sorting is required.

---

## 9. Collision Resistance

RailID uses 150 bits of random data per timestamp interval.

When multiple identifiers are generated within the same 100ms interval, collision probability follows the birthday approximation:

```text
P ≈ n(n - 1) / 2^151
```

Where:

- `n` is the number of RailIDs generated within the same Timestamp interval.
- `P` is the approximate probability of at least one collision.

The collision probability is negligible for practical generation volumes.

RailID implementations SHOULD NOT rely on collision probability alone for critical database constraints.

Database systems SHOULD enforce uniqueness on the RailID field.

---

## 10. Rust Implementation

### 10.1 Recommended Components

| Component | Responsibility |
|---|---|
| `std::time` | Timestamp acquisition |
| `tss-esapi` | TPM 2.0 interaction through TSS |
| OS CSPRNG | Secure random source |
| Bit-level encoder | Assemble 42-bit + 150-bit value |
| Base64URL encoder | Encode the 24-byte value |

The implementation SHOULD use maintained Rust crates and system TPM libraries rather than hand-implementing the TPM protocol.

### 10.2 Suggested Dependencies

```toml
[dependencies]
tss-esapi = "latest-compatible"
getrandom = "latest-compatible"
base64 = "latest-compatible"
```

Exact versions MUST be pinned and reviewed during implementation.

### 10.3 Suggested API

```rust
pub struct RailId([u8; 24]);

pub enum RailIdError {
    TimestampOutOfRange,
    TpmUnavailable,
    TpmRandomFailure,
    OsRandomFailure,
    InvalidEncoding,
    InvalidLength,
}

pub fn generate() -> Result<RailId, RailIdError>;

pub fn generate_string() -> Result<String, RailIdError>;

pub fn decode(input: &str) -> Result<RailId, RailIdError>;

pub fn timestamp(id: &RailId) -> u64;

pub fn random(id: &RailId) -> [u8; 19];

pub fn encode(id: &RailId) -> String;
```

The actual Random accessor MUST account for the fact that only 150 of the 152 bits in 19 bytes are valid.

---

## 11. Bit-Level Implementation

The 42-bit Timestamp and 150-bit Random field do not align with byte boundaries.

The implementation MUST preserve the exact bit layout.

### 11.1 Bit Layout

```text
Byte 0            Byte 5                         Byte 23
  │                  │                               │
  ┌──────────────────┬───────────────────────────────┐
  │ Timestamp 42bit  │       Random 150bit          │
  └──────────────────┴───────────────────────────────┘
                   192 bits
```

Timestamp occupies:

- Byte 0 through Byte 4
- The upper 2 bits of Byte 5

Random occupies:

- The lower 6 bits of Byte 5
- Byte 6 through Byte 23

### 11.2 Big-Endian

The 192-bit value MUST use big-endian bit ordering.

The most significant bit MUST be encoded first.

### 11.3 Implementation Safety

Rust implementations SHOULD avoid unnecessary conversions to floating point.

Timestamp and bit operations SHOULD use integer types.

The implementation MUST avoid overflow and MUST validate the Timestamp before assembly.

---

## 12. Security Considerations

### 12.1 CSPRNG

Cryptographically secure randomness is mandatory.

### 12.2 TPM Trust

TPM random output is trusted according to the security and implementation properties of the TPM hardware and its firmware.

TPM presence alone does not guarantee that the TPM, firmware, system entropy, or entire platform is secure.

### 12.3 Hybrid Randomness

The TPM and OS randomness sources SHOULD be combined.

This is intended to provide defense in depth and reduce dependence on a single entropy source.

### 12.4 Timestamp Disclosure

RailID exposes approximate creation time at 100ms precision.

Applications requiring timestamp privacy SHOULD avoid exposing RailID publicly or use a separate opaque identifier.

### 12.5 Uniqueness

RailID is designed to have an extremely low collision probability.

It is not a formal mathematical guarantee of uniqueness.

Applications SHOULD enforce uniqueness at the persistence layer.

### 12.6 Predictability

The Random field MUST NOT be derived from the Timestamp or any predictable value.

The TPM and OS CSPRNG MUST be used for random generation.

### 12.7 Replay and single use

A RailID is an identifier, not a one-time token. The format itself does not
prevent the same value from being presented more than once.

Applications that treat a RailID as a bearer token, access grant, or one-time
code MUST keep consumption state (a database uniqueness constraint, a
consumption store, or an equivalent ledger). Signatures, timestamps, or expiry
alone do not provide stateless replay protection.

---

## 13. Versioning

RailID version 1 uses the following fixed parameters:

| Parameter | v1 |
|---|---|
| Total size | 192 bits |
| Timestamp | 42 bits |
| Timestamp unit | 100ms |
| Epoch | Unix Epoch |
| Random | 150 bits |
| Random source | TPM 2.0 RNG + OS CSPRNG |
| Random combination | XOR |
| Encoding | Base64URL |
| Padding | None |
| String length | 32 characters |

Changing the bit layout, timestamp precision, randomness size, encoding alphabet, or epoch constitutes a format change and SHOULD require a new version or profile.

---

## 14. Conformance

A conforming RailID v1 implementation MUST:

- Produce a 192-bit binary identifier.
- Use a 42-bit Timestamp.
- Use a 100ms timestamp unit.
- Use a 150-bit Random field.
- Use CSPRNG-backed randomness.
- In the TPM profile, use TPM 2.0 `GetRandom`.
- Use big-endian bit ordering.
- Encode as 32-character unpadded Base64URL.
- Validate malformed input.
- Preserve the exact bit layout.
- Reject failures of required random sources.

---

## 15. Summary

RailID v1 is a 192-bit identifier with a timestamp-first design.

```text
42-bit Timestamp (100ms)
+
150-bit Random (TPM 2.0 + OS CSPRNG)
=
192-bit ID
=
24 bytes
=
32 Base64URL characters
```

RailID is intended to provide compact, URL-safe, time-aware, and highly collision-resistant identifiers for modern distributed applications.

---

## References

- TPM 2.0 Library Specification, Part 1: Architecture — Random Number Generator
- TPM 2.0 Library Specification, Part 3: Commands — `TPM2_GetRandom`
- Rust `tss-esapi` crate
- RFC 4648 — Base64URL encoding

---

## Appendix A. Implementation notes (this repository)

This appendix is not part of the format specification; it records choices made
by the Rust implementation in this repository.

1. **Reduction direction.** The 19-byte (152-bit) XOR result is shifted right by
   two bits, so the 150 *most significant* bits are retained. The two discarded
   bits are the least significant. The result is stored right-aligned in the
   accessor: `random()[0] & 0xC0 == 0`.
2. **TPM partial reads.** The TPM backend loops on `TPM2_GetRandom` until 19
   bytes are collected. Short responses are accepted and empty responses are
   retried; an oversized response, or 64 consecutive empty responses without
   progress, fails generation with `TpmRandomFailure`.
3. **TPM transport.** On Linux, `tss-esapi` is used. TCTI is selected from
   `TPM2TOOLS_TCTI` / `TCTI` / `TEST_TCTI` when set, otherwise `/dev/tpmrm0`
   then `/dev/tpm0`. The context is cached after a successful open. On Windows,
   a TPM 2.0 TBS context (`Tbsi_Context_Create` with `includeTpm20`) is created
   per thread, and `TPM2_GetRandom` commands are submitted with
   `Tbsip_Submit_Command`. TPM access is serialized within the process because a
   TPM is a single device.
4. **Feature split.** The core crate builds without system TPM libraries. The
   TPM backend lives in `railid-tpm` and is enabled by the `tpm` feature. Without
   the feature, `generate()` returns `TpmUnavailable`; it does not fall back to
   OS randomness. On targets without a TPM backend the feature is a fail-closed
   stub.
5. **Degraded mode.** `Profile::OsOnlyDegraded` and `generate_os_only()` are
   explicit opt-ins. No automatic downgrade path exists.
6. **Encoding validation.** The decoder checks length 32, the Base64URL
   alphabet, a 24-byte decode, absence of padding, and re-encodes the result to
   enforce canonical form.
7. **Windows command marshalling.** The Windows backend sends the 12-byte
   `TPM2_GetRandom` command (`TPM_ST_NO_SESSIONS`, command code `0x0000017B`,
   `UINT16 bytesRequested`) and validates the response tag, declared size,
   response code, and `TPM2B_DIGEST` size before accepting random bytes.
8. **Bulk generation.** `generate_many` is an additive API; the single-ID
   `generate` path is unchanged. The TPM bulk path requests 64 bytes per
   `TPM2_GetRandom`, slices the stream into fresh 19-byte chunks, and retains
   leftover bytes. Each ID still obtains exactly 19 TPM bytes, which may span
   two TPM responses. The OS-only bulk path uses scoped supervisor workers, not
   an ID pool; each ID's timestamp is captured at assembly time. Parallel
   OS-only results are not globally chronological across worker chunks.
9. **OS randomness buffer.** Each OS-only bulk worker uses a bounded 8 KiB
   `getrandom` buffer (`OS_RANDOM_BUFFER_BYTES`) and consumes exactly 150 bits
   per ID across byte boundaries. This avoids one OS call per ID and avoids
   discarding two bits per ID. The buffer is transient and per worker; twelve
   workers use about 96 KiB total.
10. **Streaming strings.** `for_each_many_strings` delivers strings to a
    `Fn(String) + Send + Sync` consumer inside the OS-only workers instead of
    returning a `Vec<String>`. OS-only consumers are invoked concurrently and in
    no guaranteed order; TPM consumers are invoked serially. This avoids
    cross-thread string deallocation when the consumer hashes or forwards the
    string immediately. `for_each_many_encoded` provides the allocation-free
    variant with a borrowed `&str` and a stack-allocated `EncodedRailId`.
11. **Encoded batches.** `generate_many_encoded_batch` fills one contiguous
    `Vec<u8>` with `count × 32` Base64URL characters instead of allocating one
    `String` per ID. OS-only workers write disjoint segments in parallel. This
    avoids the per-ID heap allocation that dominates `Vec<String>` throughput.
12. **Minimal dependencies.** The core crate depends only on `getrandom`. The
    fixed 32-character Base64URL form is encoded and decoded locally; the
    optional TPM backend is the separate `railid-tpm` crate.


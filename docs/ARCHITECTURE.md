# RailID Rust architecture

This document describes the Rust implementation in this repository.
The wire format is defined in [SPEC-v1.md](SPEC-v1.md); this file explains how
the code is organized and why.

## Invariants

- Binary value: 192 bits / 24 bytes.
- Timestamp: 42 bits at the most significant end, 100 ms Unix ticks.
- Random: 150 bits at the least significant end.
- Bit order: MSB-first (big-endian).
- Encoding: 32 characters, unpadded Base64URL.
- Timestamp and random are bit-aligned, not byte-aligned.

`RailId::timestamp`, `RailId::random`, and `RailId::from_parts` preserve these
invariants. `tests/conformance.rs` contains known vectors and round-trip checks.

## Module map

| Module | Responsibility |
|---|---|
| `lib.rs` | Public single-ID API, `Profile`, version, timestamp conversion |
| `id.rs` | 192-bit layout, Base64URL, `RailId`, `EncodedRailId`, `EncodedBatch` |
| `error.rs` | `RailIdError` and display/error integration |
| `rand.rs` | `RandomSource`, OS CSPRNG source, hybrid XOR/reduction |
| `tpm.rs` | Feature gate over the optional `railid-tpm` backend |
| `railid-tpm/src/lib.rs` | Platform `TPM2_GetRandom` backend (TSS ESAPI / TBS) |
| `os_random.rs` | Bounded OS randomness buffer + exact 150-bit extraction |
| `bulk.rs` | Bulk generation, supervisor workers, TPM batching, streaming/batch output |
| `bin/railid.rs` | CLI (`generate`, `generate-many`, `decode`, `inspect`) |
| `tests/conformance.rs` | Public-API conformance and behavior tests |
| `tests/cli.rs` | CLI subcommand and exit-code tests |

## Single-ID pipeline

```text
SystemTime -> 100 ms ticks
TPM2_GetRandom(19 bytes) ─┐
OS CSPRNG(19 bytes) ──────┴─ XOR -> retain upper 150 bits
   -> Timestamp[42] || Random[150]
   -> 24 bytes
   -> 32-char Base64URL
```

- `generate` uses `Profile::TpmHybrid` when the `tpm` feature is enabled;
  otherwise it returns `TpmUnavailable`.
- `generate_with_source` is the deterministic/testable core; it accepts a
  `RandomSource` and explicit ticks.
- A TPM or OS failure returns an error. There is no OS-only fallback.
- `Profile::OsOnlyDegraded` is an explicit, degraded profile.

## TPM backend

The backend is a separate crate, `railid-tpm`, so the core crate has no system
library dependency.

- Linux: `tss-esapi` 7.7, TCTI from `TPM2TOOLS_TCTI` / `TCTI` / `TEST_TCTI`,
  otherwise `/dev/tpmrm0` then `/dev/tpm0`.
- Windows: TPM Base Services (`Tbs.dll`) with `includeTpm20`.
- The TPM is a single device. Access is serialized with a mutex on both
  platforms; concurrent TPM calls would otherwise fail or queue.
- Partial `TPM2_GetRandom` responses are retried until the requested buffer is
  filled; a run of consecutive zero-byte responses is bounded. Transient TPM
  `RETRY`/`YIELDED`/`TESTING` warnings and transient TBS errors are retried a
  few times, then fail closed. A failed TSS context is dropped so a later call
  can reopen it.
- The single-ID path requests 19 bytes. Bulk generation requests 64 bytes and
  slices the stream into 19-byte chunks; leftover bytes are retained.
- Without the `tpm` feature, `railid::tpm::fill` is a stub returning
  `TpmUnavailable`.

## Bulk generation

`bulk.rs` is additive; it does not change the single-ID path.

### OS-only

- The caller is the supervisor.
- The request is split into contiguous chunks.
- One scoped worker is created per chunk; there is no worker pool and no ID
  pool.
- Each worker has an 8 KiB OS CSPRNG buffer (`OS_RANDOM_BUFFER_BYTES`).
- Exactly 150 bits are consumed per ID across byte boundaries
  (`OsBitReader`), so the two excess bits are not wasted.
- Small batches stay sequential to avoid thread-startup overhead.
- Timestamps are captured per ID at assembly time.
- The returned order is chunk order; global chronological order is not
  guaranteed across workers.

### TPM

- TPM bytes are read in 64-byte chunks and sliced into 19-byte chunks.
- A single ID still consumes 19 TPM bytes; a chunk may span two TPM responses.
- The TPM path is serialized by hardware and by the process-level mutex.
- TPM bulk is therefore throughput-limited by the device, not by worker count.

### Output forms

| API | Allocation model |
|---|---|
| `generate_many` | `Vec<RailId>`; no per-ID heap allocation |
| `generate_many_strings` | `Vec<String>`; one allocation per ID |
| `for_each_many_strings` | one owned `String` per ID, consumed in worker |
| `for_each_many_encoded` | borrowed `&str`, no per-ID allocation |
| `generate_many_encoded_batch` | one contiguous `Vec<u8>` (`count * 32`) |

Base64 arithmetic is cheap (`encode_array`); per-ID `String` allocation is the
dominant string cost. Benchmark reports are maintained in the development
workspace, not in this repository.

## Concurrency and safety

- `#![deny(unsafe_code)]` applies crate-wide except the Windows FFI module,
  which is explicitly allowed and documents its safety contracts.
- TPM handles are guarded; OS buffers are per worker.
- No global ID pool is used automatically.
- `generate_many` returns either a complete result or an error; partial batches
  are not exposed.
- Bulk OS workers are scoped threads; they terminate before the call returns.
- Generated IDs are not retained across calls; the caller owns the result.

## Features and platforms

| Feature | Default | Effect |
|---|---|---|
| `tpm` | no | Enables the optional `railid-tpm` platform backend |

Without `tpm`, `generate()` returns `TpmUnavailable`; OS-only APIs still work.
On unsupported targets the TPM backend is a fail-closed stub.

## Error model

`RailIdError` distinguishes timestamp range, TPM availability/random failure,
OS random failure, invalid encoding/length, and batch capacity overflow.
Random-source failures never become valid random data and never silently switch
profiles.

## Further reading

- [SPEC-v1.md](SPEC-v1.md): normative format and appendix.
- [BUILD.md](BUILD.md): build matrix, platform prerequisites, scripts.
- [TESTING.md](TESTING.md): test layers and hardware checks.
- [SECURITY.md](SECURITY.md): guarantees, non-goals, threat model.
- [DECISIONS.md](DECISIONS.md): architecture decision records.
- [`../README.md`](../README.md): usage overview.

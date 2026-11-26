# railid

Rust implementation of **RailID v1** (specification `1.0.0-draft`, crate
`1.0.0`): a 192-bit, timestamp-first, URL-safe identifier with cryptographically
secure randomness. This repository is MIT-licensed.

```text
42-bit timestamp (100 ms ticks, Unix Epoch)
+ 150-bit random (TPM 2.0 GetRandom XOR OS CSPRNG)
= 192 bits = 24 bytes = 32 unpadded Base64URL characters
```

The authoritative format description is in [`docs/SPEC-v1.md`](docs/SPEC-v1.md).

## Install

From crates.io:

```bash
cargo add railid                 # core, no TPM dependency
cargo add railid --features tpm  # standard TPM 2.0 profile
```

## Documentation

| Document | Contents |
|---|---|
| [`docs/SPEC-v1.md`](docs/SPEC-v1.md) | Normative format and implementation appendix |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Module map, pipelines, concurrency, safety |
| [`docs/BUILD.md`](docs/BUILD.md) | Platform prerequisites, feature matrix, scripts |
| [`docs/TESTING.md`](docs/TESTING.md) | Test layers, hardware checks, conformance coverage |
| [`docs/SECURITY.md`](docs/SECURITY.md) | Guarantees, non-goals, threat model, operational guidance |
| [`docs/DECISIONS.md`](docs/DECISIONS.md) | Architecture decision records |
| [`docs/README.md`](docs/README.md) | Documentation index and reading paths |

## Quick build

The default build has no TPM dependency and supports OS-only generation:

```bash
cargo build --release
cargo test
```

The standard TPM 2.0 profile is an opt-in feature:

```bash
cargo build --release --features tpm
cargo test --features tpm
```

On Linux, `--features tpm` needs `pkg-config` and `libtss2-dev`. On Windows it
uses the system TBS service. Without the feature, `generate()` returns
`TpmUnavailable` and never falls back silently.

Windows helper:

```powershell
.\scripts\build.ps1          # core, no TPM
.\scripts\build.ps1 -Tpm     # core + TPM backend
```

Linux/macOS helper:

```bash
./scripts/build.sh           # core, no TPM
./scripts/build.sh --tpm     # core + TPM backend
```

See [`docs/BUILD.md`](docs/BUILD.md) for details.

## Source layout

| Path | Contents |
|---|---|
| `src/lib.rs` | Public single-ID API and profile selection |
| `src/id.rs` | Bit layout, Base64URL, `RailId`, encoded types |
| `src/rand.rs` | Random sources and hybrid XOR/reduction |
| `src/tpm.rs` | Feature gate over the optional `railid-tpm` backend |
| `railid-tpm/src/lib.rs` | Platform TPM 2.0 backend (TSS / TBS) |
| `src/os_random.rs` | Bounded OS buffer and exact 150-bit extraction |
| `src/bulk.rs` | Supervisor bulk generation, streaming, encoded batches |
| `src/bin/railid.rs` | CLI |
| `tests/conformance.rs` | Public-API conformance tests |
| `tests/cli.rs` | CLI subcommand and exit-code tests |
| `docs/` | Spec, architecture, build, testing |
| `scripts/` | Build/check helpers |

## Status

- Format, timestamp, encoding, decoding, ordering, and validation: implemented.
- Default build: no TPM dependency; `generate()` returns `TpmUnavailable`.
- TPM 2.0 hybrid profile: opt-in via the `tpm` feature.
- TPM backend on Linux: `tss-esapi` 7.7 (`/dev/tpmrm0`, `/dev/tpm0`, or
  `TPM2TOOLS_TCTI` / `TCTI` / `TEST_TCTI`) in the `railid-tpm` crate.
- TPM backend on Windows: TPM Base Services (`Tbs.dll`, `Tbsi_Context_Create` +
  `Tbsip_Submit_Command`) with a TPM 2.0 context. No extra SDK or admin rights
  are required for normal `TPM2_GetRandom` use.
- Other targets: the library builds and tests, but the TPM profile returns
  `TpmUnavailable` instead of silently using OS randomness.
- Degraded OS-only generation is available only through an explicit profile or
  `generate_os_only()`.
- Bulk OS-only generation uses supervisor workers, a bounded 8 KiB OS random
  buffer per worker, and exact 150-bit extraction.
- RailID is an identifier generator, not an authentication or one-time-token
  system. Single use, expiry, and revocation belong in the application. See
  [`docs/SECURITY.md`](docs/SECURITY.md).

## Not provided

- Single-use or replay prevention
- Issuer signatures or authenticity proofs
- Revocation or expiry
- Confidentiality of creation time
- Trust in the TPM platform or boot chain
- Constant-time/side-channel guarantees

These are deliberate non-goals. If you need them, implement them at the
application layer or in a separate service.

## Layout

| Field | Bits | Position |
|---|---:|---|
| Timestamp | 42 | bits 191..150 |
| Random | 150 | bits 149..0 |

Bits are MSB-first. The timestamp occupies bytes 0..=4 and the top two bits of
byte 5. The random field occupies the low six bits of byte 5 and bytes 6..=23.

The 19-byte random accessor (`RailId::random`, `railid::random`) returns a
right-aligned 150-bit value: the two most significant bits of byte 0 are always
zero. `RailId::from_parts` likewise ignores those two bits.

## Usage

```rust
use railid::{RailId, Profile, RailIdError};

fn main() -> Result<(), RailIdError> {
    // Standard TPM profile. Requires the `tpm` feature; without it this
    // returns TpmUnavailable.
    let id = railid::generate()?;
    println!("{id}");

    // Deterministic construction (for tests, fixtures, or replay).
    let fixture = RailId::from_parts(0, [0u8; 19])?;
    assert_eq!(fixture.to_string(), "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    assert_eq!(fixture.timestamp(), 0);

    // Decode and inspect.
    let decoded = RailId::decode(&id.encode())?;
    println!(
        "ticks={} unix_ms={} random_prefix={:02x}",
        decoded.timestamp(),
        decoded.timestamp_millis(),
        decoded.random()[0]
    );

    // Explicitly degraded: OS CSPRNG only, no TPM.
    if std::env::var_os("ALLOW_DEGRADED_RAILID").is_some() {
        let degraded = railid::generate_with_profile(Profile::OsOnlyDegraded)?;
        println!("{degraded}");
    }

    Ok(())
}
```

### API summary

| Function | Purpose |
|---|---|
| `generate()` | Generate with TPM hybrid profile |
| `generate_string()` | Generate and encode |
| `generate_with_profile(Profile)` | Select TPM hybrid or explicit degraded mode |
| `generate_os_only()` | Explicit degraded mode |
| `generate_with_profile_and_time(Profile, SystemTime)` | Deterministic clock, system entropy |
| `generate_with_source(Profile, ticks, &mut RandomSource)` | Conformance tests / custom transport |
| `generate_many(count, Profile)` | Bulk generation; OS-only supervisor / TPM 64-byte reads |
| `generate_many_strings(count, Profile)` | Bulk generation plus Base64URL encoding |
| `for_each_many_strings(count, Profile, Fn(String))` | Stream owned strings to an in-worker consumer |
| `for_each_many_encoded(count, Profile, Fn(&str))` | Stream borrowed encoded strings (no allocation) |
| `generate_many_encoded_batch(count, Profile)` | One contiguous encoded batch |
| `EncodedRailId` / `EncodedBatch` | Stack / contiguous Base64URL representations |
| `bulk_worker_count(count)` | Recommended OS-only worker count |
| `RailId::from_parts(ticks, random)` | Assemble from fields |
| `RailId::timestamp()` / `timestamp_millis()` | Extract time |
| `RailId::random()` | Extract the 150-bit random field |
| `RailId::encode()` / `RailId::decode()` | Base64URL round trip |

`RailId` implements `Display`, `FromStr`, `Copy`, `Eq`, `Ord`, and `Hash`.
Binary comparison (`Ord` on `RailId` or `to_bytes()`) is chronological at
100 ms granularity. **Do not sort the encoded strings and assume chronological
order**; standard Base64URL is not order-preserving.

## Bulk generation

`generate_many(count, profile)` avoids an ID pool and creates workers only for
the duration of the call:

- **OS-only profile:** the caller is the supervisor. The request is split into
  contiguous chunks and generated by scoped worker threads. Each worker keeps a
  bounded 8 KiB OS CSPRNG buffer (`OS_RANDOM_BUFFER_BYTES`) and consumes exactly
  150 bits per ID across byte boundaries, so the two excess bits are not wasted
  and `getrandom` is called once per buffer rather than once per ID. Workers are
  capped so each chunk is around `BULK_MIN_IDS_PER_WORKER` IDs; small batches
  stay sequential to avoid thread-startup overhead. `generate_many_strings` also
  performs Base64URL encoding inside the workers.
- **TPM profile:** the TPM is a single device, so commands remain serialized.
  The bulk path requests `TPM_BULK_READ_BYTES` (64) bytes per
  `TPM2_GetRandom`, slices them into fresh 19-byte chunks, and retains leftover
  bytes for the next ID. Each RailID still receives 19 distinct TPM bytes. With
  a 64-byte read, a single TPM command can supply up to three RailIDs.

The timestamp is captured per ID when it is assembled. Parallel OS-only output
preserves chunk order but does not guarantee globally chronological order across
chunks; sort by `RailId` if needed.

Measured on the reference Windows host (real TPM 2.0, 12 logical CPUs, IDs only):

| Path | ops/sec | Note |
|---|---:|---|
| OS-only sequential loop | 16.49M | one thread |
| OS-only `generate_many` supervisor | 83.15M | 12 workers, 1M IDs, 8 KiB OS buffer per worker |
| TPM loop, 19-byte reads | 210.68 | one thread |
| TPM `generate_many`, 64-byte reads | 360.81 | 1.71x; TPM command latency grows with byte count |

Returning a materialized `Vec<String>` moves string ownership to the caller and
makes one million independent allocations and cross-thread deallocations. For
bulk string output, prefer:

- `for_each_many_encoded`: borrowed `&str` callback, no per-ID allocation,
  ~66.6M ops/s.
- `generate_many_encoded_batch`: one contiguous encoded buffer, ~56.5M ops/s.
- `for_each_many_strings`: owned `String` per ID, ~45.8M ops/s.

For comparison, `Vec<String>` reached ~15.6M ops/s and IDs-only generation
~83.2M ops/s. Node.js `crypto.randomUUID` reached ~105.0M ops/s across 12
threads when only reading `.length`; that path does not force V8 to flatten the
cons string. When the string content is actually copied into a buffer, Node
measured ~3.32M ops/s single-threaded and ~11.63M ops/s across 12 threads, while
RailID's sequential owned-string path reached ~9.25M and its encoded batch
~56.5M. Compare content-consuming modes, not `.length`-only. Base64 arithmetic
is cheap (`encode_array` ~11-17 ns); per-ID `String` allocation is the dominant
string cost.

The OS buffer is intentionally bounded: 8 KiB per worker, so twelve workers use
about 96 KiB of transient buffer memory. It is not an ID pool and is not
retained between calls.

Benchmark figures are from the development workspace. The benchmark harness and
raw artifacts are not part of this repository, and the numbers are
machine-dependent.

## TPM requirements

The conforming profile is enabled with the `tpm` feature and needs a working
TPM 2.0 plus the platform backend:

On Linux, the TSS 2.0 libraries are also required:

```bash
# Debian / Ubuntu (development headers and pkg-config)
sudo apt install pkg-config libtss2-dev
```

On Windows, the crate links to the system `Tbs.dll` through `windows-sys`. A
TPM 2.0 device must be present and enabled; no Windows SDK, custom driver, or
elevated privileges are needed for `TPM2_GetRandom`. TPM commands are serialized
within the process because a TPM is a single device.

At runtime the crate:

1. opens the platform TPM transport:
   - Linux: honours `TPM2TOOLS_TCTI`, `TCTI`, or `TEST_TCTI` when set; otherwise
     tries `/dev/tpmrm0` (kernel resource manager) and `/dev/tpm0`;
   - Windows: creates a TBS context with `includeTpm20` and submits commands
     through TBS;
2. calls `TPM2_GetRandom` and continues on partial responses until 19 bytes are
   obtained;
3. obtains 19 bytes from the OS CSPRNG independently;
4. XORs the buffers and retains the upper 150 bits.

If the TPM is unavailable or fails, generation returns an error. There is no
silent OS-only fallback, and no TPM ownership, clearing, or provisioning is
performed.

## CLI

```text
railid generate [--degraded]
railid generate-many <count> [--degraded]
railid decode <id>
railid inspect <id>
```

`--degraded` prints a warning to stderr and uses OS randomness only. The
`generate-many` command uses the same supervisor / TPM batching paths as
`generate_many`. The default binary has no TPM feature, so `generate` (without
`--degraded`) returns `TpmUnavailable`; build with `--features tpm` or install
with `cargo install railid --features tpm` for the standard profile.

## Node.js

Native N-API bindings live in [`node/`](node/README.md). The npm package name is
`railid`; it exposes sync single/batch APIs and async batch APIs.

```bash
cd node
npm run build
npm test
```

Linux TPM builds need `pkg-config` and `libtss2-dev`. The package is prepared
with TPM and OS-only binaries for Linux, TPM for Windows, and OS-only for macOS.
See [`node/README.md`](node/README.md) for the API and fallback behavior.

## Testing

```bash
cargo test                                      # core, no TPM
cargo test --features tpm                       # core + TPM backend
cargo test --manifest-path railid-tpm/Cargo.toml # backend unit tests (stub by default)
cargo clippy --all-targets --features tpm -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features tpm
```

On Linux, the TPM feature requires `pkg-config` and `libtss2-dev`. On Windows,
`cargo test --features tpm` exercises the TBS command marshalling/parsing tests
and the generation test. The generation test accepts a clean `TpmUnavailable` on
machines without a TPM; set `RAILID_REQUIRE_TPM=1` to require real hardware:

```powershell
$env:RAILID_REQUIRE_TPM = "1"
cargo test --features tpm --test conformance windows_tpm_profile_generates_or_fails_closed
```

The test suite covers known vectors, all bit boundaries, round trips, malformed
input, timestamp overflow, binary ordering, hybrid XOR, partial TPM reads, TBS
wire format, and failure propagation (including the no-fallback rule).

A Linux container can compile the TPM backend without changing the host:

```bash
docker run --rm -v "$PWD:/work" -w /work -e CARGO_TARGET_DIR=/tmp/target \
  rust:1.85-bookworm bash -c \
  'apt-get update -qq && apt-get install -y -qq pkg-config libtss2-dev >/dev/null && cargo test --features tpm'
```

See [`docs/TESTING.md`](docs/TESTING.md) for the test matrix, hardware checks,
and known gaps.

## Security notes

See [`docs/SECURITY.md`](docs/SECURITY.md) for the full threat model and
non-goals. Key points:

- In the TPM profile, randomness comes only from the TPM and OS CSPRNG.
  Timestamps, PIDs, and counters are never used as random seeds.
- A RailID discloses its creation time at 100 ms precision. Do not publish one
  where creation time is sensitive.
- Collision probability is negligible but not a mathematical guarantee. Enforce
  uniqueness in the persistence layer.
- TPM presence alone does not make a platform trustworthy; assess the TPM,
  firmware, and boot chain separately.
- The OS-only profile is degraded and is never selected automatically.
- A RailID is not a bearer token. If it grants access or credit, enforce single
  use in the application.

## Dependencies

The core `railid` crate has one direct dependency:

| Crate | Version | Role |
|---|---|---|
| `getrandom` | 0.4 | OS CSPRNG |

The optional `railid-tpm` backend adds only platform-specific TPM libraries:

| Crate | Version | Role |
|---|---|---|
| `tss-esapi` | 7.7 (Linux, `linux` feature) | TPM 2.0 `GetRandom` via TSS ESAPI |
| `windows-sys` | 0.61 (Windows, `windows` feature) | TPM 2.0 `GetRandom` via TBS |

Versions are caret requirements; `Cargo.lock` records the tested versions.

## License

This repository is MIT-licensed. Copyright (c) 2026 Nekomakura. See
[`LICENSE`](LICENSE).

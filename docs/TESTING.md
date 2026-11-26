# Testing RailID

## Test layers

| Layer | Location | Checks |
|---|---|---|
| Unit tests | `src/rand.rs`, `src/os_random.rs`, `src/tpm.rs` (Windows TBS only), `src/lib.rs` | bit reduction, buffer refills, partial/empty responses, TBS wire format, time conversion |
| Integration tests | `tests/conformance.rs`, `tests/cli.rs` | known vectors, encode/decode, length/alphabet, timestamp range, ordering, bulk, streaming, batch, CLI subcommands |
| Doc tests | `src/lib.rs` | public example compiles/runs |
| Hardware checks | `tests/conformance.rs` + manual | real TPM generation and concurrent TPM access |

The integration suite uses only the public API. The TPM transport tests on
Windows use a scripted entropy source and do not require hardware; the real TPM
tests are opt-in.

## Commands

```bash
cargo test                                      # core, no TPM dependency
cargo test --features tpm                       # core + TPM backend
cargo test --manifest-path railid-tpm/Cargo.toml # backend unit tests (stub by default)
```

On Linux, `--features tpm` requires `pkg-config` and `libtss2-dev`. The core
tests run without either.

Checks:

```bash
cargo fmt --check
cargo clippy --all-targets --features tpm -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features tpm
```

Or use `scripts/build.ps1` / `scripts/build.sh` (see [BUILD.md](BUILD.md)).

## Hardware TPM checks

Set `RAILID_REQUIRE_TPM=1` so tests that expect a TPM fail if it is missing.

```powershell
$env:RAILID_REQUIRE_TPM = "1"
cargo test --features tpm --test conformance windows_tpm_profile_generates_or_fails_closed
cargo test --features tpm --test conformance tpm_generation_is_thread_safe_when_required
Remove-Item Env:RAILID_REQUIRE_TPM
```

```bash
RAILID_REQUIRE_TPM=1 cargo test --features tpm --test conformance
```

The Windows test covers the TBS command marshalling/response parser and real
generation. The concurrency test runs 8 threads × 10 generations and verifies
that the process-level TPM serialization holds.

## Linux without a TPM

The core tests need no TSS at all:

```bash
cargo test
```

To compile and test the TPM feature with TSS installed but no device:

```bash
cargo test --features tpm
```

`generate()` should return `TpmUnavailable`; OS-only and format tests still run.
Use the Docker recipe in `../README.md` for a clean Linux build environment.

## Conformance coverage

The integration tests cover:

- 192-bit / 24-byte / 32-character invariants
- timestamp boundaries (0, max, overflow)
- random field top/bottom bit placement
- all-ones / zero known vectors
- 10,000 deterministic layout round trips
- invalid length, alphabet, padding, and non-canonical input
- binary ordering and timestamp-first comparison
- hybrid XOR/reduction
- partial TPM reads (including empty responses)
- TPM/OS failure propagation with no fallback
- bulk OS-only uniqueness and chunking
- TPM 64-byte batching and 19-byte slices
- encoding array/batch/streaming APIs
- CLI version/help/generate/decode/inspect and error exit codes

## Benchmarks

Benchmarks are not pass/fail tests and the harness is not included in this
repository. README figures are historical development measurements; re-run them
on your own hardware if you need current numbers.

## Known gaps

- Real TPM hardware tests are manual/opt-in; CI may lack a TPM.
- No continuous fuzzing target is included; malformed-input cases are covered by
  table-driven tests.
- Benchmark numbers are machine-, power-plan-, and firmware-dependent.

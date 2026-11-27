# RailID design decisions

Short architecture decision records for the Rust implementation. Each record
states the context, the decision, and the consequences. The normative wire
format remains [SPEC-v1.md](SPEC-v1.md).

## ADR-001: 192-bit timestamp-first layout

**Status:** Accepted

**Context:** IDs must be URL-safe, fixed length, sort approximately by creation
time, and carry enough randomness to avoid coordination.

**Decision:** Use 42 bits of 100 ms Unix ticks followed by 150 random bits,
MSB-first, encoded as 32 unpadded Base64URL characters. Binary comparison is
chronological because the timestamp is the most significant field.

**Consequences:** The format is compact and sortable, but the creation time is
disclosed at 100 ms precision. Encoded strings are not order-preserving, so sort
the binary value or `RailId`, not the Base64URL text.

## ADR-002: TPM 2.0 + OS CSPRNG hybrid, fail-closed

**Status:** Accepted

**Context:** A hardware RNG is desirable, but a single entropy source is a
single point of failure.

**Decision:** Fetch 19 bytes from `TPM2_GetRandom` and 19 bytes from the OS
CSPRNG independently, XOR them, and keep the upper 150 bits. If either source
fails, return an error.

**Consequences:** The output remains uniform if either source is uniform. The
standard profile is available through the `tpm` feature and then requires a
working TPM on supported platforms. Without the feature, `generate()` returns
`TpmUnavailable`; there is no silent degraded path.

## ADR-003: OS-only is explicit and degraded

**Status:** Accepted

**Context:** Some systems intentionally have no TPM, and tests need a
software-only path.

**Decision:** Expose `Profile::OsOnlyDegraded` and `generate_os_only()` as
explicit opt-ins. The standard `generate()` never selects them.

**Consequences:** Callers must make the trade-off visible in configuration and
logs. The OS-only profile is not equivalent to the standard profile.

## ADR-004: Bulk generation uses scoped supervisor workers

**Status:** Accepted

**Context:** Bulk OS-only generation can use multiple CPUs, but a long-lived
thread pool or an ID inventory would add lifecycle and memory complexity.

**Decision:** Split the request into contiguous chunks, create scoped workers
for the duration of the call, and give each worker a bounded 8 KiB OS CSPRNG
buffer with exact 150-bit extraction. TPM bulk reads 64 bytes per command and
slices them into 19-byte chunks.

**Consequences:** No global worker or ID pool is retained. Results are complete
or an error; partial batches are not exposed. OS-only chunk order is preserved,
but global chronological order is not guaranteed across workers.

## ADR-005: Remove signed, expiring ID pools

**Status:** Accepted (2026-11-25)

**Context:** An optional pool feature provided pre-generated IDs with an Ed25519
manifest, expiry, and a "single-use" contract. Audit found that a serialized
pool can be loaded twice and reissue the same IDs. This is not a bug that a
signature can fix: a stateless verifier cannot distinguish a first use from a
replay. Preventing double spend requires durable consumption state, a trusted
monotonic counter, or a shared ledger. The feature therefore conflated ID
generation with inventory and entitlement management.

**Decision:** Remove the pool module, the `pool` feature, the `ed25519-dalek`
and `sha2` dependencies, and all pool documentation. Keep RailID focused on
generation, decoding, and format validation. If an application needs single use,
expiry, or revocation, it must keep that state in its own database or
consumption service.

**Consequences:** The public API and dependency graph are smaller, and there is
no misleading "single-use" guarantee. Applications that need pre-generated
batches can still call `generate_many` and store the results themselves. A
future pool-like feature, if required, must take an explicit consumption store
and document its trust assumptions rather than claiming stateless replay
protection.

## ADR-006: Do not use a TPM-seeded DRBG for bulk throughput

**Status:** Accepted (2026-11-25)

**Context:** Per-ID `TPM2_GetRandom` is hardware-bound (roughly hundreds of
operations per second). Seeding a CSPRNG/DRBG once from the TPM and generating
many IDs from it would increase throughput.

**Decision:** Do not add a TPM-seeded DRBG. Keep the per-ID TPM + OS hybrid and
the existing 64-byte TPM bulk batching.

**Consequences:** TPM throughput remains device-bound. The implementation avoids
a weaker long-lived DRBG state and keeps the documented per-ID entropy model.
Applications that need higher throughput can use the explicitly degraded
OS-only profile or pre-generate IDs with `generate_many` and manage them
themselves.

## ADR-007: Minimal dependencies and caret requirements

**Status:** Accepted (2026-11-25)

**Context:** The wire format and entropy behavior must not drift with
uncontrolled dependency updates.

**Decision:** Keep the dependency surface as small as possible. The core crate
uses only `getrandom`; Base64URL encoding and decoding are implemented locally
for the fixed 32-character form. Use caret requirements in the published
manifests and commit a lockfile for the tested resolution.

**Consequences:** Reproducible local builds without forcing exact versions on
downstream consumers. Dependency updates are deliberate changes that require
re-running the conformance suite.

## ADR-008: Split the TPM backend and keep the core dependency-free

**Status:** Accepted (2026-11-25)

**Context:** The TPM backend needs platform system libraries (`libtss2-dev` on
Linux). If that backend is a default feature of the core crate, `cargo add
railid` fails on machines without TSS, and docs.rs needs a TPM build
environment. The backend is also the only part that needs unsafe Windows FFI.

**Decision:** Move the TPM transport into the separate `railid-tpm` crate.
`railid` is the facade: `default = []`, and `tpm = ["dep:railid-tpm",
"railid-tpm/linux", "railid-tpm/windows"]`. The core crate keeps the format,
OS-only generation, bulk generation, and a `TpmUnavailable` stub.

**Consequences:** `cargo add railid` builds anywhere. The standard profile is
explicit (`--features tpm`). Publishing requires `railid-tpm` to be released
before `railid`. The core crate has no `tss-esapi` or `windows-sys` dependency.

## ADR-009: Thread-local OS pool and cached 100 ms clock

**Status:** Accepted (2026-11-27)

**Context:** The single-ID OS-only path called `getrandom` once per ID and called
`SystemTime::now` for every timestamp. Benchmarking showed this dominated
`generate_os_only`, while the bulk path already amortized both costs.

**Decision:** Keep a thread-local 8 KiB OS byte pool for the single-ID and
hybrid OS half, and a thread-local 100 ms `TickClock` that refreshes once per
tick and never moves backwards. Bulk OS-only workers refresh the clock every
4,096 IDs. Explicit-time APIs bypass both caches. The bulk worker count remains
`available_parallelism()`; tuning on the reference 12-thread host showed 12
workers was fastest.

**Consequences:** `generate_os_only` no longer pays a per-ID OS CSPRNG call. The
timestamp remains within its 100 ms resolution. The caches are per thread, so
concurrent generation needs no lock. TPM hardware latency still dominates the
standard profile.

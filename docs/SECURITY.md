# RailID security model

This document states what RailID v1 does and does not guarantee. The normative
format is in [SPEC-v1.md](SPEC-v1.md); implementation structure is in
[ARCHITECTURE.md](ARCHITECTURE.md).

## Scope

RailID is an **identifier generator**. It produces fixed-length, time-aware,
URL-safe values with cryptographically secure randomness. It is not an
authentication protocol, an authorization system, or a one-time-token scheme.

The security of a RailID depends on how the application uses it. In particular,
an identifier can be presented more than once unless the application enforces
single use.

## Guarantees

### Unpredictability

The standard `Profile::TpmHybrid` obtains 19 bytes from the TPM 2.0
`TPM2_GetRandom` command and 19 bytes from the OS CSPRNG independently, XORs
them, and keeps the upper 150 bits. If either source is a uniform independent
random source, the XOR is uniform; compromise of one source does not by itself
make the output predictable.

The TPM profile is enabled by the `tpm` feature. Without it, `generate()`
returns `TpmUnavailable`; the default build has no system TPM dependency and
does not pretend to be the standard profile.

The 150-bit random field gives a birthday-bound collision probability of about
`n² / 2^151` for `n` IDs in the same 100 ms tick. This is negligible for normal
use, but it is probabilistic, not a mathematical uniqueness guarantee.

### Fail-closed entropy

If the TPM or the OS CSPRNG fails, `generate()` returns an error. The standard
profile never silently falls back to OS-only randomness. `OsOnlyDegraded` is an
explicit opt-in and is distinguishable at the call site.

### Format integrity

`decode()` rejects wrong lengths, non-Base64URL characters, padding, and
non-canonical spellings. This protects parsers from ambiguous encodings. It does
**not** authenticate who created an ID; `from_parts()` is public and can
construct any well-formed value.

### Timestamp semantics

The timestamp is the upper 42 bits at 100 ms resolution. It is derived from the
system clock, which can be wrong, adjusted, or rolled back. It is not a trusted
timestamp and must not be used as one. The default path caches the current tick
for at most 100 ms and never moves backwards within a thread; this is within the
format's resolution.

## Non-goals

RailID does **not** provide:

- **Single use or replay prevention.** The same ID can be used repeatedly.
- **Issuer authentication.** A RailID is not signed by the issuer.
- **Revocation or expiry.** Those require application state.
- **Confidentiality.** The creation time is visible at 100 ms precision.
- **Trust in the TPM platform.** A TPM being present does not prove that the
  firmware, OS, or TPM itself is uncompromised.
- **Constant-time or side-channel hardening.** The crate is not a general
  cryptography library and makes no such claims.

## Threat model

| Threat | Mitigation | Residual risk |
|---|---|---|
| Predictable IDs | TPM + OS CSPRNG hybrid, 150 random bits | Both entropy sources compromised |
| Silent entropy failure | Fail-closed errors, no automatic OS-only fallback | Caller explicitly selects degraded mode |
| Duplicate IDs | 150-bit random field, timestamp | Probabilistic only; enforce a unique constraint |
| Malformed input | Canonical Base64URL decoder | None for the format layer |
| Forged "issued" ID | None at this layer | Application must sign or register IDs |
| Replay / double spend | None at this layer | Application must keep consumption state |
| Stale timestamp | Caller checks time if needed | Local clock can be wrong or rolled back |
| TPM/firmware compromise | Out of scope | Assess platform and boot chain separately |
| Traffic tampering | Out of scope | Use TLS or an integrity-protected channel |

## Entropy model

```text
TPM2_GetRandom(19 bytes) ─┐
OS CSPRNG(19 bytes) ──────┴─ XOR -> keep upper 150 bits
```

- The two sources are fetched independently for every ID. No byte is reused.
- The hybrid path reduces 152 bits to 150 bits by discarding the two least
  significant bits of the XOR. Discarding bits from a uniform value does not
  introduce bias.
- Bulk OS-only generation consumes exactly 150 bits per ID from a bounded
  8 KiB buffer, including across byte boundaries, so bits are not silently
  reused or skipped.
- The OS-only profile uses only the OS CSPRNG and is explicitly degraded.

## Failure behavior

- TPM errors are returned as `TpmUnavailable` or `TpmRandomFailure`.
- OS randomness errors are returned as `OsRandomFailure`.
- Batch-returning APIs (`generate_many*`) return either a complete result or an
  error; partial batches are not exposed. Streaming APIs may have delivered
  earlier items before a later error.
- TPM access is serialized in-process. Concurrent callers queue rather than
  racing the device.
- A failed Linux TSS context is dropped so a later call can reopen it. TPM
  `RETRY`/`YIELDED`/`TESTING` warnings and transient TBS errors are retried a
  bounded number of times.

## Operational guidance

1. **Enforce uniqueness in the database.** Use a primary key or unique index on
   the encoded ID. A collision is astronomically unlikely, but the database is
   the final authority.
2. **Do not treat a RailID as a bearer token.** If an ID grants access, credit,
   or a one-time action, store its consumption state in the application. No
   stateless signature scheme can prevent replay.
3. **Do not expose creation time if it is sensitive.** A RailID reveals when it
   was created at 100 ms precision.
4. **Do not log IDs as secrets.** They are identifiers, not passwords. Protect
   them according to their role in the application.
5. **Do not enable OS-only mode silently.** Make degraded mode an explicit
   configuration choice and monitor TPM availability.
6. **Synchronize clocks if timestamp accuracy matters.** NTP improves accuracy
   but does not make the timestamp a trusted or monotonic value.
7. **Transport integrity is the application's job.** Use TLS or another
   integrity-protected channel when IDs cross a trust boundary.

## Verification

The security-relevant behavior is covered by:

- `tests/conformance.rs`: bit layout, known vectors, decode rejection, hybrid
  XOR, failure propagation, no-fallback behavior, bulk uniqueness.
- `tests/cli.rs`: CLI happy paths and error exit codes.
- Windows TBS unit tests: command marshalling and response validation.
- Hardware TPM checks with `cargo test --features tpm` and
  `RAILID_REQUIRE_TPM=1`.

See [TESTING.md](TESTING.md) for commands and known gaps.

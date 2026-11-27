# RailID documentation

This directory is the entry point for the RailID implementation. The normative
wire format is [SPEC-v1.md](SPEC-v1.md); the code in `../src/` is the reference
implementation.

## Reading paths

| If you want to... | Read |
|---|---|
| Use the crate or CLI | [../README.md](../README.md) → [BUILD.md](BUILD.md) |
| Implement the format independently | [SPEC-v1.md](SPEC-v1.md) |
| Understand the code | [ARCHITECTURE.md](ARCHITECTURE.md) → [SPEC-v1.md](SPEC-v1.md) |
| Operate or deploy it | [BUILD.md](BUILD.md) → [SECURITY.md](SECURITY.md) |
| Review security assumptions | [SECURITY.md](SECURITY.md) → [DECISIONS.md](DECISIONS.md) |
| Run or extend tests | [TESTING.md](TESTING.md) |

## Document map

| Document | Contents |
|---|---|
| [SPEC-v1.md](SPEC-v1.md) | Normative 192-bit format, encoding, validation, implementation appendix |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Module map, single-ID/bulk pipelines, TPM backends, concurrency, safety |
| [BUILD.md](BUILD.md) | Toolchain, platform prerequisites, feature matrix, scripts, troubleshooting |
| [TESTING.md](TESTING.md) | Test layers, hardware TPM checks, conformance coverage, known gaps |
| [SECURITY.md](SECURITY.md) | Guarantees, non-goals, threat model, entropy model, operational guidance |
| [DECISIONS.md](DECISIONS.md) | ADRs, including pool removal and the decision not to use a TPM-seeded DRBG |
| [../README.md](../README.md) | Usage, API summary, CLI, license |

## Source of truth

- **Format:** [SPEC-v1.md](SPEC-v1.md).
- **Behavior:** the Rust code plus `../tests/conformance.rs` and
  `../tests/cli.rs`. If documentation and code disagree, treat the tests and
  the public API as the current behavior and file a documentation fix.
- **Performance:** benchmark figures in the README are historical development
  measurements. The benchmark harness and raw artifacts are not part of this
  repository.
- **History:** [../CHANGELOG.md](../CHANGELOG.md) and git history.

## Related projects

- [`../railid-tpm/`](../railid-tpm/) — optional TPM 2.0 backend crate enabled by
  the `tpm` feature.
- [`../node/`](../node/) — native N-API bindings published as
  `@neko-makura/railid`.

## Conventions

- Paths are relative to this directory unless stated otherwise.
- "Must"/"must not" are normative only in [SPEC-v1.md](SPEC-v1.md); other
  documents explain implementation choices.
- The crate version and the specification draft version are separate. The crate
  is `1.2.0`; the specification header may still say `1.0.0-draft`. `railid`,
  `railid-tpm` and the npm package share the project version.

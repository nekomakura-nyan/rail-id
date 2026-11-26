# Building and running RailID

## Toolchain

- Rust **1.85+** (edition 2021) for the core crate
- Cargo with `rustfmt` and `clippy` for checks
- Windows or Linux for the TPM backend

Related crate MSRVs: `railid` 1.85, `railid-tpm` 1.85.

Dependencies use caret requirements; `Cargo.lock` records the tested versions.

## Platform prerequisites

The core crate has no platform-specific build dependency. TPM prerequisites
apply only when `--features tpm` is enabled.

### Windows

- TPM 2.0 enabled and available through TBS.
- No Windows SDK, custom driver, or administrator rights are required for
  `TPM2_GetRandom`.
- The TPM profile uses `railid-tpm` / `windows-sys` / `Tbs.dll`.

### Linux

```bash
sudo apt install pkg-config libtss2-dev
```

- TPM access normally needs membership in the `tss` group, or root.
- The backend tries `TPM2TOOLS_TCTI` / `TCTI` / `TEST_TCTI` first, then
  `/dev/tpmrm0`, then `/dev/tpm0`.
- Without a TPM, build the core without `--features tpm`; `generate()` then
  returns `TpmUnavailable`.

### macOS / other targets

The core crate builds and OS-only generation works. The TPM feature is compiled
as a fail-closed stub and returns `TpmUnavailable`.

## Feature matrix

| Features | TPM backend | Use |
|---|---|---|
| default (none) | no | core format + OS-only generation |
| `--features tpm` | platform | standard TPM 2.0 hybrid profile |

```bash
cargo build --release
cargo build --release --features tpm
cargo test
cargo test --features tpm
cargo test --manifest-path railid-tpm/Cargo.toml
```

## Build and check scripts

From the crate directory:

```powershell
# Windows
.\scripts\build.ps1
.\scripts\build.ps1 -Tpm
.\scripts\build.ps1 -SkipTests
```

```bash
# Linux / macOS
./scripts/build.sh
./scripts/build.sh --tpm
```

`-NoTpm` / `--no-tpm` remain accepted as compatibility no-ops; the default
build has no TPM dependency.

The scripts run, in order:

```text
cargo fmt --check
cargo clippy --all-targets ... -- -D warnings
cargo test ...
cargo build --release ...
RUSTDOCFLAGS=-D warnings cargo doc --no-deps ...
```

## Manual commands

```bash
cargo fmt --check
cargo clippy --all-targets --features tpm -- -D warnings
cargo test
cargo test --features tpm
cargo test --manifest-path railid-tpm/Cargo.toml
cargo build --release
cargo build --release --features tpm
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features tpm
```

`--features tpm` is the only optional feature. On Linux it requires the TSS
development package.

## CLI

```bash
# Core: OS-only is available without the TPM feature.
cargo run --release -- generate --degraded
cargo run --release -- generate-many 100000 --degraded

# TPM profile:
cargo run --release --features tpm -- generate
cargo run --release --features tpm -- generate-many 1000

cargo run --release -- decode <id>
cargo run --release -- inspect <id>
cargo run --release -- --version
```

The release binary is `target/release/railid` (`.exe` on Windows). Install the
TPM-enabled CLI with `cargo install railid --features tpm`.

## Related Rust crate

| Crate | Path | Build |
|---|---|---|
| `railid-tpm` | `railid-tpm/` | `cargo build --manifest-path railid-tpm/Cargo.toml --features linux` (or `windows`) |

`railid-tpm` is the optional backend enabled by `railid`'s `tpm` feature.

## Troubleshooting

| Symptom | Check |
|---|---|
| `TPM 2.0 is unavailable` on Windows | TPM enabled in firmware/Windows; TBS accessible |
| `TPM 2.0 is unavailable` on Linux | `/dev/tpmrm0`, `tss` group, TCTI variables |
| `tss-esapi` build fails on Linux | `pkg-config` and `libtss2-dev` installed |
| TPM calls fail under concurrency | Expected: the device is serialized; use bulk APIs |
| TPM batch is slow | Hardware-bound; 64-byte reads are already used |
| String throughput low | Avoid `Vec<String>`; use encoded stream/batch |

Do not silently switch to OS-only on TPM failure. The standard profile is
fail-closed by design.

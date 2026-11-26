# railid

RailID v1: a 192-bit, timestamp-first, URL-safe identifier with
cryptographically secure randomness.

```text
42-bit timestamp (100 ms ticks, Unix Epoch)
+ 150-bit random (TPM 2.0 GetRandom XOR OS CSPRNG)
= 192 bits = 24 bytes = 32 unpadded Base64URL characters
```

The crate is an identifier generator. It does not provide authentication,
single-use tokens, revocation, or expiry; those belong in the application.

## Install

Core (OS CSPRNG only; `generate()` returns `TpmUnavailable`):

```toml
[dependencies]
railid = "1.0"
```

Standard TPM 2.0 profile:

```toml
[dependencies]
railid = { version = "1.0", features = ["tpm"] }
```

The `tpm` feature on Linux requires `pkg-config` and `libtss2-dev`; Windows uses
the system TBS service and needs no SDK.

## Usage

```rust
use railid::RailId;

fn main() -> Result<(), railid::RailIdError> {
    // Standard profile (requires the `tpm` feature).
    let id = railid::generate()?;
    println!("{id}");

    // Explicitly degraded OS-only profile.
    let os_id = railid::generate_os_only()?;

    // Deterministic construction for fixtures and tests.
    let fixture = RailId::from_parts(0, [0u8; 19])?;
    assert_eq!(fixture.to_string(), "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

    // Decode and inspect.
    let decoded = RailId::decode(&id.encode())?;
    println!(
        "ticks={} unix_ms={} random_prefix={:02x}",
        decoded.timestamp(),
        decoded.timestamp_millis(),
        decoded.random()[0]
    );

    let _ = os_id;
    Ok(())
}
```

## Features

| Feature | Default | Effect |
|---|---|---|
| `tpm` | no | Enables the `railid-tpm` backend for the TPM 2.0 hybrid profile |

Without `tpm`, `generate()` returns `TpmUnavailable` and never silently falls
back to OS randomness. `generate_os_only()` is the explicit degraded profile.

## Bulk generation

- `generate_many` / `generate_many_strings`
- `for_each_many_strings` / `for_each_many_encoded`
- `generate_many_encoded_batch`

OS-only bulk generation uses scoped supervisor workers and a bounded 8 KiB OS
randomness buffer per worker. TPM bulk generation reads 64 bytes per
`TPM2_GetRandom` and slices them into 19-byte chunks.

## Security notes

- A RailID discloses its creation time at 100 ms precision.
- Uniqueness is probabilistic; enforce a unique constraint in persistence.
- A RailID is not a bearer token. Replay prevention requires application state.
- TPM presence does not prove the firmware or boot chain is trustworthy.
- The OS-only profile is degraded and is never selected automatically.

Full documentation: <https://docs.rs/railid>.

## Source

<https://github.com/nekomakura-nyan/rail-id>

## License

MIT. Copyright (c) 2026 Nekomakura.

# railid-tpm

TPM 2.0 `GetRandom` backend for [RailID](https://crates.io/crates/railid).

This crate is normally used through the `tpm` feature of the `railid` crate:

```toml
[dependencies]
railid = { version = "1.0", features = ["tpm"] }
```

It exists as a separate crate so the core `railid` build has no system
library dependency. Direct use is possible but not recommended; the public
surface is `fill()` and `TpmError`.

## Platforms

| Platform | Backend | Build requirement |
|---|---|---|
| Linux | TSS ESAPI (`tss-esapi`) | `pkg-config` and `libtss2-dev` |
| Windows | TBS (`Tbs.dll`) | TPM 2.0 enabled; no SDK required |
| Other | fail-closed stub | none |

On Linux, enable the `linux` feature; on Windows, enable `windows`. The
`railid` crate's `tpm` feature enables the appropriate one for the target.

## Behavior

- Partial `TPM2_GetRandom` responses are retried until the caller's buffer is
  filled.
- A run of consecutive empty responses is bounded.
- TPM `RETRY` / `YIELDED` / `TESTING` warnings and transient TBS errors are
  retried a few times.
- A failed TSS context or TBS handle is dropped so a later call can reopen it.
- There is no fallback to the OS CSPRNG.

## License

MIT. Copyright (c) 2026 Nekomakura. This repository is MIT-licensed.

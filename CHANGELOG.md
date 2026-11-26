# Changelog

## 1.0.0 - 2026-11-25

- Initial public release of `railid` and `railid-tpm` on crates.io.
- 192-bit timestamp-first RailID v1 format with unpadded Base64URL encoding.
- TPM 2.0 + OS CSPRNG hybrid profile, fail-closed; explicit OS-only degraded
  profile.
- Bulk generation, streaming string APIs, and contiguous encoded batches.
- `railid` core has one direct dependency (`getrandom`); the optional
  `railid-tpm` backend contains the Linux TSS and Windows TBS transports.
- MIT license for the `railid` and `railid-tpm` crates.

## 2026-11-26

- Moved the public source to the standalone `rail-id` repository.
- Made the documentation self-contained: benchmark harness and Node.js addon
  are no longer referenced as part of this repository.

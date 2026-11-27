# Changelog

## 1.1.0 - 2026-11-27

- Release `railid` 1.1.0. `railid-tpm` is unchanged and remains 1.0.0.
- Node package `railid` starts at 1.1.0; it was not published at 1.0.0.
- perf(core): added a thread-local 8 KiB OS byte pool for single-ID
  `generate_os_only` / hybrid OS half. `generate_os_only` dropped from about
  201 ns to 154 ns per ID in the Node benchmark.
- perf(core): added a thread-local 100 ms `TickClock`; single-ID and bulk-worker
  timestamps refresh at most once per tick and never move backwards. Bulk OS-only
  loops refresh the clock every 4,096 IDs; explicit time APIs bypass the cache.
- perf(core): retuned bulk worker measurement. On the 6-core/12-thread reference
  host, 12 workers remained fastest for 1M encoded IDs (93.3 Mops/s tuned run),
  so `available_parallelism()` keeps the default.
- feat(node): added `generateBuffer()` and `generateOsOnlyBuffer()` returning
  32 ASCII bytes without a JS string. A per-call Buffer allocation is slower
  than the string path; batch APIs remain the throughput path.
- perf(node): `generate`, `generateOsOnly`, `generateMany`, and async string
  batches now build JS strings directly with `napi_create_string_latin1` from
  the stack, avoiding a Rust `String` allocation and UTF-8 conversion.
- bench(node): final results: RailID OS-only 9.34 Mops/s single, 37.34 Mops/s at
  12 workers; string array 129.22 ns/ID; encoded batch 93.58 Mops/s; NanoID
  12.43 single / 56.39 at 12 workers; UUIDv4 3.40 / 13.22. See the `*-final.md`
  result files.

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
- feat(node): added native N-API bindings as npm package `railid` 1.0.0.
  Sync APIs (`generate`, `generateOsOnly`, `generateMany`,
  `generateManyEncoded`, `decode`, `version`) and async bulk APIs
  (`generateManyAsync`, `generateManyEncodedAsync`) are included.
- Linux ships both a TPM binary (requires `libtss2`) and an OS-only binary;
  `index.js` falls back to OS-only when the TPM binary cannot be loaded.
  Windows ships TPM, macOS ships OS-only.
- Added a GitHub Actions matrix for Linux x64, Windows x64, macOS x64/arm64,
  and npm tarball assembly. **npm publication is intentionally left for the
  final review step.**
- bench(node): added `node/bench/` comparing native RailID with `uuid` 14.0.3
  and `nanoid` 6.0.2 under forced string materialization. Local Windows result:
  UUIDv4 1.52 Mops/s, NanoID 6.31 Mops/s, RailID OS-only 2.90 Mops/s, RailID
  TPM 213 ops/s, encoded batch 29.6-36.4 Mops/s. See
  `node/bench/results/20261127-windows-amd64-native-vs-uuid-nanoid.md`.
- bench(node): added `worker_threads` scaling (1/2/4/6/8/12 workers). On the
  6-core/12-thread reference host: NanoID 6.10 -> 36.11 Mops/s (5.91x), UUIDv4
  1.39 -> 8.75 Mops/s (6.29x), RailID OS-only 2.68 -> 18.32 Mops/s (6.85x).
  See `node/bench/results/20261127-windows-amd64-multithread-scaling.md`.
- bench(node): clean rerun after stopping background load. Single ID: UUIDv4
  3.23 Mops/s, NanoID 12.42 Mops/s, RailID OS-only 4.97 Mops/s, TPM 219 ops/s,
  encoded batch 68.5-73.9 Mops/s. 12-worker scaling: NanoID 54.69 Mops/s
  (4.57x), UUIDv4 13.31 Mops/s (4.04x), RailID OS-only 26.42 Mops/s (5.20x).
  See the `*-clean.md` result files.
- bench(node): added NanoID component analysis. `getRandomValues(21)` costs
  ~1.13 us/call; a 32 KiB pool refill costs ~39.5 us and covers ~1,560 IDs
  (~25 ns/ID). Forced-copy cost: NanoID 93 ns, UUIDv4 303 ns, RailID OS-only
  213 ns. See `node/bench/results/20261127-windows-amd64-nanoid-analysis.md`.
- bench(node): added batch comparison. NanoID/UUIDv4 have no bulk API, so their
  rows are JS loops. JS string arrays: NanoID 7.06 Mops/s, RailID 6.22 Mops/s,
  UUIDv4 1.93 Mops/s. Flat 1M buffers: RailID encoded batch 49.98 Mops/s,
  NanoID 21 packed 11.44 Mops/s, UUIDv4 36 packed 3.34 Mops/s. See
  `node/bench/results/20261127-windows-amd64-batch-comparison.md`.


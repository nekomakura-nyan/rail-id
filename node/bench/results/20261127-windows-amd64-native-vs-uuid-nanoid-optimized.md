# RailID native vs UUIDv4 / NanoID

Generated: 2026-11-27T03:07:48.184Z

## Environment

| Item | Value |
|---|---|
| OS | Windows_NT 10.0.26200 (win32 x64) |
| CPU | AMD Ryzen 5 8500G w/ Radeon 740M Graphics       |
| Logical CPUs | 12 |
| Node.js | v24.19.0 |
| uuid | 14.0.3 |
| nanoid | 6.0.2 |
| RailID native backend | tpm |
| TPM | available |

## Method

- Single-ID rows: 200,000 iterations per sample, 7 samples, 3 warmups.
- TPM row: 100 iterations per sample, 5 samples, 2 warmups.
- Batch rows: 1,000,000 IDs per sample, 5 samples, 2 warmups.
- Every string result is written into a fixed ASCII buffer before the timer stops, so V8 cannot skip materialization.
- Encoded batch rows also scan every 32-byte ID boundary.
- Throughput is `1 / median per-ID time`; batch rows divide the call by ID count.

## Results

| Workload | Output | Median/ID | Throughput | Relative to UUIDv4 |
|---|---:|---:|---:|---:|
| UUIDv4 (uuid) | string | 282.37 ns | 3.54 Mops/s | 1.00x |
| NanoID 21 | string | 79.38 ns | 12.6 Mops/s | 3.56x faster |
| RailID OS-only single | string | 153.69 ns | 6.51 Mops/s | 1.84x faster |
| RailID OS-only single Buffer | 32-byte Buffer | 656.53 ns | 1.52 Mops/s | 2.33x slower |
| RailID TPM single | string | 4.77 ms | 209.52 ops/s | 16,902.98x slower |
| RailID OS-only encoded batch | 32-byte slice | 15.06 ns | 66.4 Mops/s | 18.75x faster |
| RailID OS-only encoded batch async | 32-byte slice | 15.49 ns | 64.56 Mops/s | 18.23x faster |

## Notes

- UUIDv4 outputs 36 hyphenated characters (122 random bits); NanoID outputs 21 characters (about 126 bits); RailID outputs 32 Base64URL characters (150 random bits).
- `uuid` 14.0.3 uses Node.js `crypto.randomUUID` for the default v4 path on Node.
- RailID OS-only is the explicitly degraded profile; the TPM row is the standard hardware profile.
- The single-ID rows include the native N-API call, string allocation, and a forced ASCII copy.
- The TPM row is hardware-bound and serialized by the TPM device.
- Batch rows use a different API: one native call returns a contiguous encoded Buffer, then every ID boundary is read.
- Checksum: -1313744060

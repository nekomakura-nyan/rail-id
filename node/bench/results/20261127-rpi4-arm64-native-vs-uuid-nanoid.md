# RailID native vs UUIDv4 / NanoID

Generated: 2026-11-27T14:36:06.078Z

## Environment

| Item | Value |
|---|---|
| OS | Linux 7.0.0-1023-raspi (linux arm64) |
| CPU | Cortex-A72 |
| Logical CPUs | 4 |
| Node.js | v22.22.1 |
| uuid | 14.0.3 |
| nanoid | 6.0.2 |
| RailID native backend | os-only |
| TPM | unavailable: TPM 2.0 is unavailable |

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
| UUIDv4 (uuid) | string | 2.57 us | 389.07 kops/s | 1.00x |
| NanoID 21 | string | 875.3 ns | 1.14 Mops/s | 2.94x faster |
| RailID OS-only single | string | 788.88 ns | 1.27 Mops/s | 3.26x faster |
| RailID OS-only single Buffer | 32-byte Buffer | 7.12 us | 140.5 kops/s | 2.77x slower |
| RailID OS-only encoded batch | 32-byte slice | 106.34 ns | 9.4 Mops/s | 24.17x faster |
| RailID OS-only encoded batch async | 32-byte slice | 130.48 ns | 7.66 Mops/s | 19.7x faster |

## Notes

- UUIDv4 outputs 36 hyphenated characters (122 random bits); NanoID outputs 21 characters (about 126 bits); RailID outputs 32 Base64URL characters (150 random bits).
- `uuid` 14.0.3 uses Node.js `crypto.randomUUID` for the default v4 path on Node.
- RailID OS-only is the explicitly degraded profile; the TPM row is the standard hardware profile.
- The single-ID rows include the native N-API call, string allocation, and a forced ASCII copy.
- The TPM row is hardware-bound and serialized by the TPM device.
- Batch rows use a different API: one native call returns a contiguous encoded Buffer, then every ID boundary is read.
- Checksum: -1313990715

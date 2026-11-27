# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T03:06:10.440Z

| Item | Value |
|---|---|
| OS | Windows_NT 10.0.26200 (win32 x64) |
| CPU | AMD Ryzen 5 8500G w/ Radeon 740M Graphics       |
| Logical CPUs | 12 |
| Node.js | v24.19.0 |
| IDs per sample | 1,000,000 |
| Samples / warmups | 5 / 2 |

| Workload | Workers | Median/ID | Total throughput | Scaling vs 1 worker |
|---|---:|---:|---:|---:|
| NanoID 21 | 1 | 90.26 ns | 11.08 Mops/s | 1.00x |
| NanoID 21 | 2 | 46.98 ns | 21.28 Mops/s | 1.92x |
| NanoID 21 | 4 | 29.78 ns | 33.58 Mops/s | 3.03x |
| NanoID 21 | 6 | 23.63 ns | 42.31 Mops/s | 3.82x |
| NanoID 21 | 8 | 19.89 ns | 50.27 Mops/s | 4.54x |
| NanoID 21 | 12 | 18.75 ns | 53.34 Mops/s | 4.81x |
| UUIDv4 (uuid) | 1 | 311.34 ns | 3.21 Mops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 191.46 ns | 5.22 Mops/s | 1.63x |
| UUIDv4 (uuid) | 4 | 118.90 ns | 8.41 Mops/s | 2.62x |
| UUIDv4 (uuid) | 6 | 97.60 ns | 10.25 Mops/s | 3.19x |
| UUIDv4 (uuid) | 8 | 87.90 ns | 11.38 Mops/s | 3.54x |
| UUIDv4 (uuid) | 12 | 78.91 ns | 12.67 Mops/s | 3.95x |
| RailID OS-only | 1 | 164.07 ns | 6.09 Mops/s | 1.00x |
| RailID OS-only | 2 | 87.20 ns | 11.47 Mops/s | 1.88x |
| RailID OS-only | 4 | 53.14 ns | 18.82 Mops/s | 3.09x |
| RailID OS-only | 6 | 41.53 ns | 24.08 Mops/s | 3.95x |
| RailID OS-only | 8 | 36.73 ns | 27.23 Mops/s | 4.47x |
| RailID OS-only | 12 | 36.46 ns | 27.43 Mops/s | 4.50x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69646074514

# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T02:12:17.459Z

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
| NanoID 21 | 1 | 83.61 ns | 11.96 Mops/s | 1.00x |
| NanoID 21 | 2 | 45.21 ns | 22.12 Mops/s | 1.85x |
| NanoID 21 | 4 | 29.64 ns | 33.74 Mops/s | 2.82x |
| NanoID 21 | 6 | 22.55 ns | 44.36 Mops/s | 3.71x |
| NanoID 21 | 8 | 21.13 ns | 47.33 Mops/s | 3.96x |
| NanoID 21 | 12 | 18.29 ns | 54.69 Mops/s | 4.57x |
| UUIDv4 (uuid) | 1 | 303.38 ns | 3.30 Mops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 161.11 ns | 6.21 Mops/s | 1.88x |
| UUIDv4 (uuid) | 4 | 107.02 ns | 9.34 Mops/s | 2.83x |
| UUIDv4 (uuid) | 6 | 90.74 ns | 11.02 Mops/s | 3.34x |
| UUIDv4 (uuid) | 8 | 88.81 ns | 11.26 Mops/s | 3.42x |
| UUIDv4 (uuid) | 12 | 75.16 ns | 13.31 Mops/s | 4.04x |
| RailID OS-only | 1 | 196.96 ns | 5.08 Mops/s | 1.00x |
| RailID OS-only | 2 | 110.39 ns | 9.06 Mops/s | 1.78x |
| RailID OS-only | 4 | 70.58 ns | 14.17 Mops/s | 2.79x |
| RailID OS-only | 6 | 55.97 ns | 17.87 Mops/s | 3.52x |
| RailID OS-only | 8 | 44.23 ns | 22.61 Mops/s | 4.45x |
| RailID OS-only | 12 | 37.85 ns | 26.42 Mops/s | 5.20x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69650752291

# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T06:14:21.009Z

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
| NanoID 21 | 1 | 80.72 ns | 12.39 Mops/s | 1.00x |
| NanoID 21 | 2 | 45.82 ns | 21.82 Mops/s | 1.76x |
| NanoID 21 | 4 | 29.45 ns | 33.96 Mops/s | 2.74x |
| NanoID 21 | 6 | 23.38 ns | 42.78 Mops/s | 3.45x |
| NanoID 21 | 8 | 20.36 ns | 49.11 Mops/s | 3.96x |
| NanoID 21 | 12 | 21.63 ns | 46.23 Mops/s | 3.73x |
| UUIDv4 (uuid) | 1 | 349.61 ns | 2.86 Mops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 269.68 ns | 3.71 Mops/s | 1.30x |
| UUIDv4 (uuid) | 4 | 142.86 ns | 7.00 Mops/s | 2.45x |
| UUIDv4 (uuid) | 6 | 125.87 ns | 7.94 Mops/s | 2.78x |
| UUIDv4 (uuid) | 8 | 100.73 ns | 9.93 Mops/s | 3.47x |
| UUIDv4 (uuid) | 12 | 92.72 ns | 10.79 Mops/s | 3.77x |
| RailID OS-only | 1 | 119.20 ns | 8.39 Mops/s | 1.00x |
| RailID OS-only | 2 | 68.43 ns | 14.61 Mops/s | 1.74x |
| RailID OS-only | 4 | 92.77 ns | 10.78 Mops/s | 1.28x |
| RailID OS-only | 6 | 64.30 ns | 15.55 Mops/s | 1.85x |
| RailID OS-only | 8 | 54.88 ns | 18.22 Mops/s | 2.17x |
| RailID OS-only | 12 | 25.38 ns | 39.40 Mops/s | 4.70x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69649402708

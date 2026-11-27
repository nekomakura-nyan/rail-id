# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T06:23:55.570Z

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
| NanoID 21 | 1 | 82.40 ns | 12.14 Mops/s | 1.00x |
| NanoID 21 | 2 | 52.53 ns | 19.04 Mops/s | 1.57x |
| NanoID 21 | 4 | 29.37 ns | 34.05 Mops/s | 2.81x |
| NanoID 21 | 6 | 22.34 ns | 44.77 Mops/s | 3.69x |
| NanoID 21 | 8 | 20.64 ns | 48.46 Mops/s | 3.99x |
| NanoID 21 | 12 | 17.73 ns | 56.39 Mops/s | 4.65x |
| UUIDv4 (uuid) | 1 | 301.26 ns | 3.32 Mops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 163.23 ns | 6.13 Mops/s | 1.85x |
| UUIDv4 (uuid) | 4 | 110.38 ns | 9.06 Mops/s | 2.73x |
| UUIDv4 (uuid) | 6 | 92.99 ns | 10.75 Mops/s | 3.24x |
| UUIDv4 (uuid) | 8 | 83.09 ns | 12.04 Mops/s | 3.63x |
| UUIDv4 (uuid) | 12 | 75.62 ns | 13.22 Mops/s | 3.98x |
| RailID OS-only | 1 | 108.79 ns | 9.19 Mops/s | 1.00x |
| RailID OS-only | 2 | 58.02 ns | 17.23 Mops/s | 1.87x |
| RailID OS-only | 4 | 37.43 ns | 26.71 Mops/s | 2.91x |
| RailID OS-only | 6 | 34.14 ns | 29.29 Mops/s | 3.19x |
| RailID OS-only | 8 | 29.08 ns | 34.39 Mops/s | 3.74x |
| RailID OS-only | 12 | 26.78 ns | 37.34 Mops/s | 4.06x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69648172149

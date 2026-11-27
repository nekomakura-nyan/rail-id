# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T01:56:16.472Z

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
| NanoID 21 | 1 | 163.81 ns | 6.10 Mops/s | 1.00x |
| NanoID 21 | 2 | 88.73 ns | 11.27 Mops/s | 1.85x |
| NanoID 21 | 4 | 47.98 ns | 20.84 Mops/s | 3.41x |
| NanoID 21 | 6 | 34.68 ns | 28.84 Mops/s | 4.72x |
| NanoID 21 | 8 | 27.28 ns | 36.66 Mops/s | 6.01x |
| NanoID 21 | 12 | 27.70 ns | 36.11 Mops/s | 5.91x |
| UUIDv4 (uuid) | 1 | 719.34 ns | 1.39 Mops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 413.28 ns | 2.42 Mops/s | 1.74x |
| UUIDv4 (uuid) | 4 | 226.69 ns | 4.41 Mops/s | 3.17x |
| UUIDv4 (uuid) | 6 | 159.02 ns | 6.29 Mops/s | 4.52x |
| UUIDv4 (uuid) | 8 | 140.74 ns | 7.11 Mops/s | 5.11x |
| UUIDv4 (uuid) | 12 | 114.34 ns | 8.75 Mops/s | 6.29x |
| RailID OS-only | 1 | 373.74 ns | 2.68 Mops/s | 1.00x |
| RailID OS-only | 2 | 207.91 ns | 4.81 Mops/s | 1.80x |
| RailID OS-only | 4 | 107.22 ns | 9.33 Mops/s | 3.49x |
| RailID OS-only | 6 | 80.55 ns | 12.41 Mops/s | 4.64x |
| RailID OS-only | 8 | 61.44 ns | 16.28 Mops/s | 6.08x |
| RailID OS-only | 12 | 54.60 ns | 18.32 Mops/s | 6.85x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69650083879

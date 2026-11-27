# NanoID / UUIDv4 / RailID OS-only multi-thread scaling

Generated: 2026-11-27T14:37:41.456Z

| Item | Value |
|---|---|
| OS | Linux 7.0.0-1023-raspi (linux arm64) |
| CPU | Cortex-A72 |
| Logical CPUs | 4 |
| Node.js | v22.22.1 |
| IDs per sample | 1,000,000 |
| Samples / warmups | 5 / 2 |

| Workload | Workers | Median/ID | Total throughput | Scaling vs 1 worker |
|---|---:|---:|---:|---:|
| NanoID 21 | 1 | 767.29 ns | 1.30 Mops/s | 1.00x |
| NanoID 21 | 2 | 416.27 ns | 2.40 Mops/s | 1.84x |
| NanoID 21 | 4 | 235.16 ns | 4.25 Mops/s | 3.26x |
| NanoID 21 | 6 | 241.45 ns | 4.14 Mops/s | 3.18x |
| NanoID 21 | 8 | 241.26 ns | 4.14 Mops/s | 3.18x |
| NanoID 21 | 12 | 243.01 ns | 4.12 Mops/s | 3.16x |
| UUIDv4 (uuid) | 1 | 2310.70 ns | 432.77 kops/s | 1.00x |
| UUIDv4 (uuid) | 2 | 1357.38 ns | 736.71 kops/s | 1.70x |
| UUIDv4 (uuid) | 4 | 785.50 ns | 1.27 Mops/s | 2.94x |
| UUIDv4 (uuid) | 6 | 838.12 ns | 1.19 Mops/s | 2.76x |
| UUIDv4 (uuid) | 8 | 826.24 ns | 1.21 Mops/s | 2.80x |
| UUIDv4 (uuid) | 12 | 834.11 ns | 1.20 Mops/s | 2.77x |
| RailID OS-only | 1 | 721.38 ns | 1.39 Mops/s | 1.00x |
| RailID OS-only | 2 | 381.13 ns | 2.62 Mops/s | 1.89x |
| RailID OS-only | 4 | 192.32 ns | 5.20 Mops/s | 3.75x |
| RailID OS-only | 6 | 194.88 ns | 5.13 Mops/s | 3.70x |
| RailID OS-only | 8 | 197.14 ns | 5.07 Mops/s | 3.66x |
| RailID OS-only | 12 | 198.93 ns | 5.03 Mops/s | 3.63x |

## Notes

- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.
- Worker startup and teardown are excluded from the timed rounds.
- Round time includes worker message dispatch and result collection.
- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.
- Checksum: 69648509849

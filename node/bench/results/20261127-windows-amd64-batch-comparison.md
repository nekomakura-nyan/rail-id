# Batch comparison: NanoID / UUIDv4 / RailID

| Workload | N | Output | Median/ID | Throughput |
|---|---:|---|---:|---:|
| NanoID 21 string array | 100,000 | string | 141.73 ns | 7.06 Mops/s |
| NanoID 21 packed Buffer | 1,000,000 | buffer21 | 87.39 ns | 11.44 Mops/s |
| NanoID 32 packed Buffer | 1,000,000 | buffer32 | 106.48 ns | 9.39 Mops/s |
| UUIDv4 string array | 100,000 | string | 517.88 ns | 1.93 Mops/s |
| UUIDv4 packed Buffer | 1,000,000 | buffer36 | 299.57 ns | 3.34 Mops/s |
| RailID OS-only string array | 100,000 | string | 160.86 ns | 6.22 Mops/s |
| RailID OS-only encoded batch | 1,000,000 | buffer32 | 20.01 ns | 49.98 Mops/s |

## Notes

- NanoID and UUIDv4 have no bulk API; their batch rows are JavaScript loops, included for a like-for-like per-ID comparison.
- NanoID string array uses 100,000 IDs; packed-buffer rows use 1,000,000 IDs.
- Packed NanoID/UUID rows include the JS loop, string creation, and Buffer.write per ID.
- RailID encoded batch is one N-API call returning a contiguous 32-byte-per-ID Buffer.
- NanoID 21 is 21 bytes/ID (126 bits); NanoID 32 is 32 bytes/ID (192 bits, 6 bits per character); RailID is 32 bytes/ID (150 bits + timestamp); UUIDv4 is 36 bytes/ID.
- Throughput is 1 / median per-ID time.
- Sink: -877470970

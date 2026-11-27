# Node native benchmark

Compares the native RailID addon with `uuid` v4 and `nanoid` on the same
Node.js process, and measures multi-thread scaling with `worker_threads`.

## Prerequisites

- Node.js 22+ for the benchmark dependencies (`nanoid` 6 requires Node 22/24)
- A built `railid` addon in `node/` (`npm run build`)
- `npm install` in this directory

## Single-process benchmark

```bash
cd node/bench
npm install
npm run bench
```

Write a result file:

```bash
npm run bench > results/YYYYMMDD-host-native-vs-uuid-nanoid.md
```

On Windows PowerShell, `>` writes UTF-16; use `cmd /c "npm run bench > results\..."` or convert the file to UTF-8 afterward.

## Multi-thread benchmark

```bash
node multithread.mjs > results/YYYYMMDD-host-multithread-scaling.md
```

Worker counts: 1, 2, 4, 6, 8, 12. Worker startup and teardown are excluded
from the timed rounds; message dispatch is included.

Latest local results:

- [`results/20261127-windows-amd64-native-vs-uuid-nanoid.md`](results/20261127-windows-amd64-native-vs-uuid-nanoid.md)
- [`results/20261127-windows-amd64-multithread-scaling.md`](results/20261127-windows-amd64-multithread-scaling.md)

## Method

- Every string result is written into a fixed ASCII buffer before the timer
  stops, so V8 cannot skip materialization.
- Single-ID rows use 200,000 iterations per sample, 7 samples, and 3 warmups.
- The TPM row uses 100 iterations per sample, 5 samples, and 2 warmups.
- Batch rows generate 1,000,000 IDs per sample and scan every 32-byte boundary.
- Multi-thread rows use 1,000,000 IDs per round, 5 samples, and 2 warmups.
- Throughput is `1 / median per-ID time`.

## Why NanoID is fast single-threaded

NanoID 6 amortizes randomness:

- `crypto.getRandomValues` fills a pool of up to 32 KiB, not one call per ID.
- The default 64-character alphabet is a power of two, so it uses a bitmask and
  needs no rejection sampling.
- It pre-converts the pool to a Latin-1 string and returns IDs with
  `substring`, avoiding per-character concatenation.
- It has no timestamp, TPM round-trip, XOR, or bit-packing step.
- It outputs 21 characters (about 126 random bits), versus RailID's 32.

`uuid` 14.0.3 uses Node's `crypto.randomUUID` per call, which is fast but not
pooled; that explains much of the UUID/NanoID gap in this benchmark.

## Multi-thread results

On the reference 6-core / 12-thread Ryzen 5 8500G:

| Workload | 1 worker | 12 workers | Scaling |
|---|---:|---:|---:|
| NanoID 21 | 6.10 Mops/s | 36.11 Mops/s | 5.91x |
| UUIDv4 (uuid) | 1.39 Mops/s | 8.75 Mops/s | 6.29x |
| RailID OS-only | 2.68 Mops/s | 18.32 Mops/s | 6.85x |

NanoID scales near the physical-core limit (6 cores), then flattens at 8-12
logical workers. Each worker has its own NanoID pool. RailID OS-only parallel
throughput is lower because the single-ID API reads the OS CSPRNG per ID; use
RailID's bulk APIs for higher throughput.

## Caveats

- UUIDv4 (36 chars, 122 random bits), NanoID 21 (about 126 bits), and RailID
  (32 chars, 150 bits) have different output sizes and entropy budgets.
- RailID OS-only is the explicitly degraded profile; TPM is the standard
  hardware profile and is serialized by the device.
- Batch rows use a different API: one native call returns a contiguous encoded
  buffer. They are useful as a throughput reference, not as a direct replacement
  for per-ID UUID/NanoID calls.
- All numbers are machine-, OS-, Node-, and load-dependent.

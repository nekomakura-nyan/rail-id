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

Latest local results (clean run after stopping background load):

- [`results/20261127-windows-amd64-native-vs-uuid-nanoid-clean.md`](results/20261127-windows-amd64-native-vs-uuid-nanoid-clean.md)
- [`results/20261127-windows-amd64-multithread-scaling-clean.md`](results/20261127-windows-amd64-multithread-scaling-clean.md)
- [`results/20261127-windows-amd64-nanoid-analysis.md`](results/20261127-windows-amd64-nanoid-analysis.md) — component cost breakdown

Earlier runs under background load are kept for comparison:

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
| NanoID 21 | 11.96 Mops/s | 54.69 Mops/s | 4.57x |
| UUIDv4 (uuid) | 3.30 Mops/s | 13.31 Mops/s | 4.04x |
| RailID OS-only | 5.08 Mops/s | 26.42 Mops/s | 5.20x |

NanoID scales to about 4.6x on this 6-core / 12-thread host; the remaining gap
is scheduler, message dispatch, and GC overhead. Each worker has its own NanoID
pool. RailID OS-only parallel throughput is lower because the single-ID API
reads the OS CSPRNG per ID; use RailID's bulk APIs for higher throughput.

## Cost breakdown

`nanoid-analysis.mjs` isolates the expensive parts:

| Operation | Median |
|---|---:|
| `crypto.randomUUID` `.length` only | 37.83 ns |
| `crypto.randomUUID` forced copy | 249.99 ns |
| `uuid.v4` `.length` only | 56.09 ns |
| `uuid.v4` forced copy | 302.85 ns |
| `nanoid` `.length` only | 34.80 ns |
| `nanoid` forced copy | 93.40 ns |
| RailID OS-only `.length` only | 179.41 ns |
| RailID OS-only forced copy | 213.25 ns |
| `crypto.getRandomValues(21)` per call | 1,132.27 ns |
| prebuilt pool substring + copy | 53.89 ns |
| 32 KiB pool refill | 39,500 ns (25.32 ns/ID at 1,560 IDs) |

This shows the two NanoID advantages:

1. **RNG call amortization.** A per-call `getRandomValues(21)` is ~1.1 us on this
   host, while a 32 KiB pool refill costs ~39.5 us and covers ~1,560 IDs
   (25.3 ns/ID).
2. **Cheap string materialization.** NanoID copies in ~93 ns; UUID takes
   ~303 ns when its result is forced into a buffer. `crypto.randomUUID` alone
   is only ~38 ns when V8 can leave it as a lazy/rope string, but forcing the
   content exposes the flattening cost.

NanoID's multi-thread scaling is similarly simple: each worker gets its own
pool and there is no shared state. The workload is CPU-bound after the refill,
so it scales until physical cores and memory bandwidth saturate.

## Caveats

- UUIDv4 (36 chars, 122 random bits), NanoID 21 (about 126 bits), and RailID
  (32 chars, 150 bits) have different output sizes and entropy budgets.
- RailID OS-only is the explicitly degraded profile; TPM is the standard
  hardware profile and is serialized by the device.
- Batch rows use a different API: one native call returns a contiguous encoded
  buffer. They are useful as a throughput reference, not as a direct replacement
  for per-ID UUID/NanoID calls.
- All numbers are machine-, OS-, Node-, and load-dependent.

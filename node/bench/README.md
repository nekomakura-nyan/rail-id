# Node native benchmark

Compares the native RailID addon with `uuid` v4 and `nanoid` on the same
Node.js process.

## Prerequisites

- Node.js 18+
- A built `railid` addon in `node/` (`npm run build`)
- `npm install` in this directory

## Run

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

The latest local result is
[`results/20261127-windows-amd64-native-vs-uuid-nanoid.md`](results/20261127-windows-amd64-native-vs-uuid-nanoid.md).

## Method

- Every string result is written into a fixed ASCII buffer before the timer
  stops, so V8 cannot skip materialization.
- Single-ID rows use 200,000 iterations per sample, 7 samples, and 3 warmups.
- The TPM row uses 100 iterations per sample, 5 samples, and 2 warmups.
- Batch rows generate 1,000,000 IDs per sample and scan every 32-byte boundary.
- Throughput is `1 / median per-ID time`.

## Caveats

- UUIDv4 (36 chars, 122 random bits), NanoID 21 (about 126 bits), and RailID
  (32 chars, 150 bits) have different output sizes and entropy budgets.
- `uuid` 14.0.3 uses Node's `crypto.randomUUID` for the default v4 path.
- RailID OS-only is the explicitly degraded profile; TPM is the standard
  hardware profile and is serialized by the device.
- Batch rows use a different API: one native call returns a contiguous encoded
  buffer. They are useful as a throughput reference, not as a direct replacement
  for per-ID UUID/NanoID calls.
- All numbers are machine-, OS-, Node-, and load-dependent.

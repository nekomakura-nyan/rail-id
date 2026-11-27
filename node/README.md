# @neko-makura/railid (Node.js)

Native N-API bindings for [RailID](https://crates.io/crates/railid).

```bash
npm install @neko-makura/railid
```

The package includes prebuilt N-API binaries. Node.js 18 or newer is required.

## Platforms

| Platform | Architecture | libc | Backend |
|---|---|---|---|
| Linux | x64 | glibc | TPM (`libtss2`) + OS-only fallback |
| Linux | x64 | musl | OS-only |
| Linux | arm64 | glibc | OS-only |
| Linux | arm64 | musl | OS-only |
| Linux | armv7 | glibc (hard-float) | OS-only |
| Linux | riscv64 | glibc | OS-only |
| Windows | x64 | MSVC | TPM (TBS) |
| Windows | arm64 | MSVC | TPM (TBS) |
| macOS | x64 | — | OS-only |
| macOS | arm64 | — | OS-only |

`index.js` selects the addon from `process.platform`, `process.arch` and, on
Linux, whether the Node process runs on glibc or musl.

The riscv64 addon targets the `riscv64gc` glibc ABI used by distribution Node
builds (for example Ubuntu 24.04). Node.js does not publish official riscv64
binaries; use a distribution package.


## Usage

```js
const railid = require('@neko-makura/railid');

console.log(railid.version());
console.log(railid.backend()); // "tpm" or "os-only"

// Standard TPM hybrid profile. Throws if the backend is OS-only or no TPM
// device is available.
const id = railid.generate();
console.log(id);

// Explicitly degraded OS-only profile.
const osId = railid.generateOsOnly();

// Synchronous batch.
const ids = railid.generateMany(1000, true);
const batch = railid.generateManyEncoded(1000, true);
// batch is a Buffer of 1000 * 32 ASCII bytes.

// Asynchronous batch: runs on the libuv thread pool.
const asyncIds = await railid.generateManyAsync(1000, true);
const asyncBatch = await railid.generateManyEncodedAsync(1000, true);

const decoded = railid.decode(id);
console.log(decoded.timestampMs, decoded.randomHex, decoded.encoded);
```

## API

| Function | Result | Notes |
|---|---|---|
| `generate()` | `string` | TPM hybrid profile; throws `TpmUnavailable` if unavailable |
| `generateOsOnly()` | `string` | Explicit degraded profile |
| `generateBuffer()` | `Buffer` | TPM profile as 32 ASCII bytes, no JS string |
| `generateOsOnlyBuffer()` | `Buffer` | OS-only as 32 ASCII bytes, no JS string |
| `generateMany(count, osOnly?)` | `string[]` | Sync; `osOnly` defaults to `false` |
| `generateManyEncoded(count, osOnly?)` | `Buffer` | Sync; `count * 32` bytes |
| `generateManyAsync(count, osOnly?)` | `Promise<string[]>` | Bulk work off the JS thread |
| `generateManyEncodedAsync(count, osOnly?)` | `Promise<Buffer>` | Bulk work off the JS thread |
| `decode(input)` | `{ timestampMs, randomHex, encoded }` | Validates canonical Base64URL |
| `version()` | `string` | RailID crate version |
| `backend()` | `"tpm" \| "os-only"` | Backend compiled into the loaded binary |

Limits:

- OS-only strings: 100,000 IDs
- OS-only encoded: 1,000,000 IDs
- TPM: 1,024 IDs

Counts above the limit are rejected before allocation.

`generateBuffer()` avoids the JS string but still allocates a Node Buffer per
call; for throughput use `generateManyEncoded()` or
`generateManyEncodedAsync()`, which allocate one contiguous buffer.

## Caller-provided buffer API (design, not shipped)

A zero-allocation single-ID API would look like:

```ts
generateInto(target: Buffer, offset?: number): number;
generateOsOnlyInto(target: Buffer, offset?: number): number;
```

- `offset` defaults to 0 and must satisfy `offset + 32 <= target.length`.
- The function writes 32 ASCII bytes and returns the end offset.
- It performs no allocation and no JS string creation.
- It is synchronous only.

This is **not shipped** because N-API buffers can be shared with other JS
execution contexts, and napi-rs documents mutable `Buffer` access as an aliasing
hazard. Shipping it safely needs an exclusive-ownership guarantee, a rejection
of `SharedArrayBuffer`-backed buffers, or a custom writer object. Until then,
use the batch APIs.

## Linux and TPM

The Linux x64 glibc TPM binary links against `libtss2` (`tss-esapi`). The
package ships two Linux x64 glibc binaries:

- `railid-tpm.linux-x64-gnu.node` — TPM backend; needs `libtss2`
- `railid.linux-x64-gnu.node` — OS-only; no TSS dependency

`index.js` tries the TPM binary first and falls back to the OS-only binary when
the TPM binary cannot be loaded (for example, when `libtss2` is missing). In
that case `generate()` throws `TpmUnavailable`; `generateOsOnly()` still works.
This preserves fail-closed behavior.

Debian/Ubuntu TPM users need:

```bash
sudo apt install libtss2
```

Alpine, arm64, armv7 and riscv64 ship the OS-only backend only. Those
platforms load `railid.<platform>-<arch>[-<libc>].node` directly, so `generate()`
throws `TpmUnavailable` and `generateOsOnly()` is the supported profile.

Windows uses the system TBS service (`Tbs.dll`) and needs no extra DLL. macOS
ships the OS-only backend.

## Build from source

Requirements:

- Rust 1.88+
- Linux TPM build: `pkg-config`, `libtss2-dev`
- Node.js 18+

```bash
npm run build          # host default
npm run build -- --os-only
npm run build -- --target linux-arm64-musl   # cross target, OS-only
npm run verify         # check every .node binary against its target tag
npm test
```

`node scripts/build.js --list` prints the supported targets. On Linux the
default build produces both TPM and OS-only binaries for x64; other targets
produce the OS-only binary. On Windows it produces the TPM binary. On macOS it
produces both OS-only slices.

Cross-compiling without a matching host is easiest in a container. The helper
installs the toolchain for the requested target:

```bash
docker run --rm -v "$PWD:/src" -w /src/node rust:1.88-alpine \
  sh scripts/build-in-docker.sh linux-arm64-musl
```

## Benchmarks

The Node addon benchmark against UUIDv4 and NanoID is in
[`bench/`](bench/README.md). It includes the latest Windows result.

## Security notes

RailID is an identifier generator. This package does not provide single-use
tokens, expiry, or revocation; enforce those in your application. A RailID
discloses its creation time at 100 ms precision. TPM presence does not prove
the firmware or boot chain is trustworthy.

TPM access is serialized inside the process. Use the async batch APIs for
server workloads so the event loop is not blocked by large batches.

## License

MIT. Copyright (c) 2026 Nekomakura. See [LICENSE](LICENSE).

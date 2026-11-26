# railid (Node.js)

Native N-API bindings for [RailID](https://crates.io/crates/railid).

```bash
npm install railid
```

The package includes prebuilt N-API binaries. Node.js 18 or newer is required.

## Usage

```js
const railid = require('railid');

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

## Linux and TPM

The Linux TPM binary links against `libtss2` (`tss-esapi`). The package ships
two Linux x64 binaries:

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

Windows uses the system TBS service (`Tbs.dll`) and needs no extra DLL. macOS
ships the OS-only backend.

## Build from source

Requirements:

- Rust 1.88+
- Linux TPM build: `pkg-config`, `libtss2-dev`
- Node.js 18+

```bash
npm run build          # platform default
npm run build -- --os-only
npm test
```

On Linux the default build produces both TPM and OS-only binaries. On Windows
it produces the TPM binary. On macOS it produces the OS-only binary.

## Security notes

RailID is an identifier generator. This package does not provide single-use
tokens, expiry, or revocation; enforce those in your application. A RailID
discloses its creation time at 100 ms precision. TPM presence does not prove
the firmware or boot chain is trustworthy.

TPM access is serialized inside the process. Use the async batch APIs for
server workloads so the event loop is not blocked by large batches.

## License

MIT. Copyright (c) 2026 Nekomakura. See [LICENSE](LICENSE).

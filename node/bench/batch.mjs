import { createRequire } from 'node:module';
import { v4 as uuidv4 } from 'uuid';
import { customAlphabet, nanoid } from 'nanoid';

const require = createRequire(import.meta.url);
const railid = require('../index.js');

const STRING_COUNT = 100_000;
const BUFFER_COUNT = 1_000_000;
const SAMPLES = 5;
const WARMUPS = 2;
const nano32 = customAlphabet(
  '57X6y3wQZNzYjJB2L_10pxbackgroundhimselfUTF-8DEMOGRAPHICSVqvt4KW9',
  32
);

let sink = 0;
const scratch = Buffer.alloc(128);

function scanBuffer(buffer, stride) {
  for (let offset = 0; offset < buffer.length; offset += stride) {
    sink = (sink + buffer[offset] + buffer[offset + stride - 1]) | 0;
  }
}

function consumeStrings(values) {
  for (const value of values) {
    const length = scratch.write(value, 0, 'ascii');
    sink = (sink + scratch[0] + scratch[length - 1]) | 0;
  }
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function measure(name, count, output, fn) {
  for (let index = 0; index < WARMUPS; index++) fn(count);

  const samples = [];
  for (let index = 0; index < SAMPLES; index++) {
    const started = process.hrtime.bigint();
    const value = fn(count);
    if (Buffer.isBuffer(value)) {
      scanBuffer(value, output === 'buffer32' ? 32 : output === 'buffer21' ? 21 : 36);
    } else {
      consumeStrings(value);
    }
    const elapsed = Number(process.hrtime.bigint() - started);
    samples.push(elapsed / count);
  }
  return { name, count, output, nsPerId: median(samples) };
}

const nanoid21 = nanoid;
const nanoid32 = nano32;

const rows = [
  measure('NanoID 21 string array', STRING_COUNT, 'string', (count) => {
    const values = new Array(count);
    for (let index = 0; index < count; index++) values[index] = nanoid21();
    return values;
  }),
  measure('NanoID 21 packed Buffer', BUFFER_COUNT, 'buffer21', (count) => {
    const output = Buffer.allocUnsafe(count * 21);
    for (let index = 0; index < count; index++) {
      output.write(nanoid21(), index * 21, 'latin1');
    }
    return output;
  }),
  measure('NanoID 32 packed Buffer', BUFFER_COUNT, 'buffer32', (count) => {
    const output = Buffer.allocUnsafe(count * 32);
    for (let index = 0; index < count; index++) {
      output.write(nanoid32(), index * 32, 'latin1');
    }
    return output;
  }),
  measure('UUIDv4 string array', STRING_COUNT, 'string', (count) => {
    const values = new Array(count);
    for (let index = 0; index < count; index++) values[index] = uuidv4();
    return values;
  }),
  measure('UUIDv4 packed Buffer', BUFFER_COUNT, 'buffer36', (count) => {
    const output = Buffer.allocUnsafe(count * 36);
    for (let index = 0; index < count; index++) {
      output.write(uuidv4(), index * 36, 'latin1');
    }
    return output;
  }),
  measure('RailID OS-only string array', STRING_COUNT, 'string', (count) =>
    railid.generateMany(count, true)
  ),
  measure('RailID OS-only encoded batch', BUFFER_COUNT, 'buffer32', (count) =>
    railid.generateManyEncoded(count, true)
  ),
];

console.log('# Batch comparison: NanoID / UUIDv4 / RailID');
console.log('');
console.log('| Workload | N | Output | Median/ID | Throughput |');
console.log('|---|---:|---|---:|---:|');
for (const row of rows) {
  console.log(
    `| ${row.name} | ${row.count.toLocaleString()} | ${row.output} | ${row.nsPerId.toFixed(2)} ns | ${(1000 / row.nsPerId).toFixed(2)} Mops/s |`
  );
}
console.log('');
console.log('## Notes');
console.log('');
console.log('- NanoID and UUIDv4 have no bulk API; their batch rows are JavaScript loops, included for a like-for-like per-ID comparison.');
console.log('- NanoID string array uses 100,000 IDs; packed-buffer rows use 1,000,000 IDs.');
console.log('- Packed NanoID/UUID rows include the JS loop, string creation, and Buffer.write per ID.');
console.log('- RailID encoded batch is one N-API call returning a contiguous 32-byte-per-ID Buffer.');
console.log('- NanoID 21 is 21 bytes/ID (126 bits); NanoID 32 is 32 bytes/ID (192 bits, 6 bits per character); RailID is 32 bytes/ID (150 bits + timestamp); UUIDv4 is 36 bytes/ID.');
console.log('- Throughput is 1 / median per-ID time.');
console.log(`- Sink: ${sink}`);

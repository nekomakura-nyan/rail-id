import { createRequire } from 'node:module';
import crypto from 'node:crypto';
import { v4 as uuidv4 } from 'uuid';
import { nanoid } from 'nanoid';

const require = createRequire(import.meta.url);
const railid = require('../index.js');

const ITERATIONS = 500_000;
const SAMPLES = 5;
const WARMUPS = 2;
const scratch = Buffer.alloc(128);
const random21 = Buffer.allocUnsafe(21);
const pool = 'A'.repeat(32_768);
let poolOffset = 0;
let sink = 0;

function consumeCopy(value) {
  const length = scratch.write(value, 0, 'ascii');
  sink = (sink + scratch[0] + scratch[length - 1]) | 0;
}

function consumeLength(value) {
  sink = (sink + value.length) | 0;
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function measure(name, fn, iterations = ITERATIONS) {
  for (let index = 0; index < WARMUPS; index++) {
    for (let operation = 0; operation < iterations; operation++) fn();
  }
  const samples = [];
  for (let index = 0; index < SAMPLES; index++) {
    const started = process.hrtime.bigint();
    for (let operation = 0; operation < iterations; operation++) fn();
    samples.push(Number(process.hrtime.bigint() - started) / iterations);
  }
  return { name, ns: median(samples) };
}

function measureRefill() {
  const refills = 1_000;
  const perRefill = [];
  for (let sample = 0; sample < SAMPLES; sample++) {
    const started = process.hrtime.bigint();
    for (let refill = 0; refill < refills; refill++) {
      const buffer = Buffer.allocUnsafe(32_768);
      crypto.getRandomValues(buffer);
      for (let index = 0; index < buffer.length; index++) {
        buffer[index] = 65 + (buffer[index] & 63);
      }
      sink += buffer.toString('latin1').length;
    }
    perRefill.push(Number(process.hrtime.bigint() - started) / refills);
  }
  return { name: '32 KiB pool refill (per refill)', ns: median(perRefill) };
}

const rows = [
  measure('crypto.randomUUID length only', () => consumeLength(crypto.randomUUID())),
  measure('crypto.randomUUID copy', () => consumeCopy(crypto.randomUUID())),
  measure('uuid.v4 length only', () => consumeLength(uuidv4())),
  measure('uuid.v4 copy', () => consumeCopy(uuidv4())),
  measure('nanoid length only', () => consumeLength(nanoid())),
  measure('nanoid copy', () => consumeCopy(nanoid())),
  measure('RailID OS-only length only', () => consumeLength(railid.generateOsOnly())),
  measure('RailID OS-only copy', () => consumeCopy(railid.generateOsOnly())),
  measure('getRandomValues(21) only', () => {
    crypto.getRandomValues(random21);
    sink = (sink + random21[0]) | 0;
  }),
  measure('prebuilt pool substring copy', () => {
    const start = poolOffset;
    poolOffset = (poolOffset + 21) % (pool.length - 21);
    consumeCopy(pool.substring(start, start + 21));
  }),
];
const refill = measureRefill();

console.log('# NanoID cost breakdown');
console.log('');
console.log('| Operation | Median |');
console.log('|---|---:|');
for (const row of rows) {
  console.log(`| ${row.name} | ${row.ns.toFixed(2)} ns |`);
}
console.log(`| ${refill.name} | ${refill.ns.toFixed(2)} ns (${(refill.ns / 1560).toFixed(2)} ns/ID at 1,560 IDs/refill) |`);
console.log('');
console.log(`- Iterations per sample: ${ITERATIONS.toLocaleString()}, samples: ${SAMPLES}, warmups: ${WARMUPS}.`);
console.log('- `length only` may not force V8 to flatten the string; `copy` writes into a fixed ASCII buffer.');
console.log(`- Sink: ${sink}`);

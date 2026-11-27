import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import os from 'node:os';
import { v4 as uuidv4 } from 'uuid';
import { nanoid } from 'nanoid';

const require = createRequire(import.meta.url);
const railid = require('../index.js');

const UUID_VERSION = JSON.parse(
  readFileSync(new URL('./node_modules/uuid/package.json', import.meta.url), 'utf8')
).version;
const NANOID_VERSION = JSON.parse(
  readFileSync(new URL('./node_modules/nanoid/package.json', import.meta.url), 'utf8')
).version;

const CONFIG = {
  singleIterations: 200_000,
  singleSamples: 7,
  singleWarmups: 3,
  tpmIterations: 100,
  tpmSamples: 5,
  tpmWarmups: 2,
  batchCount: 1_000_000,
  batchSamples: 5,
  batchWarmups: 2,
  asyncStringCount: 100_000,
};

const scratch = Buffer.alloc(128);
let sink = 0;

function consumeString(value) {
  const length = scratch.write(value, 0, 'ascii');
  sink = (sink + scratch[0] + scratch[length - 1]) | 0;
}

function consumeEncodedBatch(batch) {
  for (let offset = 0; offset < batch.length; offset += 32) {
    sink = (sink + batch[offset] + batch[offset + 31]) | 0;
  }
}

function consumeSingleBuffer(buffer) {
  sink = (sink + buffer[0] + buffer[31]) | 0;
}

function timeOnce(fn) {
  const started = process.hrtime.bigint();
  fn();
  return Number(process.hrtime.bigint() - started);
}

async function timeOnceAsync(fn) {
  const started = process.hrtime.bigint();
  await fn();
  return Number(process.hrtime.bigint() - started);
}

function percentile(sorted, fraction) {
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[Math.max(0, index)];
}

async function measure(name, operations, iterations, samples, warmups, fn) {
  for (let index = 0; index < warmups; index++) {
    fn();
  }

  const perOperation = [];
  const denominator = iterations * operations;
  for (let index = 0; index < samples; index++) {
    const elapsed = timeOnce(() => {
      for (let operation = 0; operation < iterations; operation++) {
        fn();
      }
    });
    perOperation.push(elapsed / denominator);
  }
  perOperation.sort((left, right) => left - right);
  const median = perOperation[Math.floor(perOperation.length / 2)];
  const mean = perOperation.reduce((sum, value) => sum + value, 0) / perOperation.length;
  return {
    name,
    operations,
    iterations,
    samples,
    medianNs: median,
    meanNs: mean,
    p95Ns: percentile(perOperation, 0.95),
    opsPerSec: 1e9 / median,
  };
}

async function measureAsync(name, operations, samples, warmups, fn) {
  for (let index = 0; index < warmups; index++) {
    await fn();
  }

  const perOperation = [];
  for (let index = 0; index < samples; index++) {
    const elapsed = await timeOnceAsync(fn);
    perOperation.push(elapsed / operations);
  }
  perOperation.sort((left, right) => left - right);
  const median = perOperation[Math.floor(perOperation.length / 2)];
  const mean = perOperation.reduce((sum, value) => sum + value, 0) / perOperation.length;
  return {
    name,
    operations,
    iterations: 1,
    samples,
    medianNs: median,
    meanNs: mean,
    p95Ns: percentile(perOperation, 0.95),
    opsPerSec: 1e9 / median,
  };
}

function formatNumber(value) {
  return value.toLocaleString('en-US', { maximumFractionDigits: 2 });
}

function formatRate(value) {
  if (value >= 1e6) {
    return `${formatNumber(value / 1e6)} Mops/s`;
  }
  if (value >= 1e3) {
    return `${formatNumber(value / 1e3)} kops/s`;
  }
  return `${formatNumber(value)} ops/s`;
}

function formatNs(value) {
  if (value >= 1e6) {
    return `${formatNumber(value / 1e6)} ms`;
  }
  if (value >= 1e3) {
    return `${formatNumber(value / 1e3)} us`;
  }
  return `${formatNumber(value)} ns`;
}

const tpmAvailable = (() => {
  try {
    consumeString(railid.generate());
    return true;
  } catch (error) {
    return `${error.message}`;
  }
})();

const rows = [];

rows.push(
  await measure(
    'UUIDv4 (uuid)',
    1,
    CONFIG.singleIterations,
    CONFIG.singleSamples,
    CONFIG.singleWarmups,
    () => consumeString(uuidv4())
  )
);

rows.push(
  await measure(
    'NanoID 21',
    1,
    CONFIG.singleIterations,
    CONFIG.singleSamples,
    CONFIG.singleWarmups,
    () => consumeString(nanoid())
  )
);

rows.push(
  await measure(
    'RailID OS-only single',
    1,
    CONFIG.singleIterations,
    CONFIG.singleSamples,
    CONFIG.singleWarmups,
    () => consumeString(railid.generateOsOnly())
  )
);

rows.push(
  await measure(
    'RailID OS-only single Buffer',
    1,
    CONFIG.singleIterations,
    CONFIG.singleSamples,
    CONFIG.singleWarmups,
    () => consumeSingleBuffer(railid.generateOsOnlyBuffer())
  )
);

if (tpmAvailable === true) {
  rows.push(
    await measure(
      'RailID TPM single',
      1,
      CONFIG.tpmIterations,
      CONFIG.tpmSamples,
      CONFIG.tpmWarmups,
      () => consumeString(railid.generate())
    )
  );
}

rows.push(
  await measure(
    'RailID OS-only encoded batch',
    CONFIG.batchCount,
    1,
    CONFIG.batchSamples,
    CONFIG.batchWarmups,
    () => consumeEncodedBatch(railid.generateManyEncoded(CONFIG.batchCount, true))
  )
);

rows.push(
  await measureAsync(
    'RailID OS-only encoded batch async',
    CONFIG.batchCount,
    CONFIG.batchSamples,
    CONFIG.batchWarmups,
    async () => consumeEncodedBatch(await railid.generateManyEncodedAsync(CONFIG.batchCount, true))
  )
);

const uuidRow = rows[0];
const cpu = os.cpus()[0]?.model ?? 'unknown';

console.log('# RailID native vs UUIDv4 / NanoID');
console.log('');
console.log(`Generated: ${new Date().toISOString()}`);
console.log('');
console.log('## Environment');
console.log('');
console.log('| Item | Value |');
console.log('|---|---|');
console.log(`| OS | ${os.type()} ${os.release()} (${process.platform} ${process.arch}) |`);
console.log(`| CPU | ${cpu} |`);
console.log(`| Logical CPUs | ${os.cpus().length} |`);
console.log(`| Node.js | ${process.version} |`);
console.log(`| uuid | ${UUID_VERSION} |`);
console.log(`| nanoid | ${NANOID_VERSION} |`);
console.log(`| RailID native backend | ${railid.backend()} |`);
console.log(`| TPM | ${tpmAvailable === true ? 'available' : `unavailable: ${tpmAvailable}`} |`);
console.log('');
console.log('## Method');
console.log('');
console.log(`- Single-ID rows: ${CONFIG.singleIterations.toLocaleString()} iterations per sample, ${CONFIG.singleSamples} samples, ${CONFIG.singleWarmups} warmups.`);
console.log(`- TPM row: ${CONFIG.tpmIterations} iterations per sample, ${CONFIG.tpmSamples} samples, ${CONFIG.tpmWarmups} warmups.`);
console.log(`- Batch rows: ${CONFIG.batchCount.toLocaleString()} IDs per sample, ${CONFIG.batchSamples} samples, ${CONFIG.batchWarmups} warmups.`);
console.log('- Every string result is written into a fixed ASCII buffer before the timer stops, so V8 cannot skip materialization.');
console.log('- Encoded batch rows also scan every 32-byte ID boundary.');
console.log('- Throughput is `1 / median per-ID time`; batch rows divide the call by ID count.');
console.log('');
console.log('## Results');
console.log('');
console.log('| Workload | Output | Median/ID | Throughput | Relative to UUIDv4 |');
console.log('|---|---:|---:|---:|---:|');
for (const row of rows) {
  const output = row.name.includes('batch')
    ? '32-byte slice'
    : row.name.includes('Buffer')
      ? '32-byte Buffer'
      : 'string';
  const relative = row.opsPerSec / uuidRow.opsPerSec;
  const relativeText = row.name.startsWith('UUIDv4')
    ? '1.00x'
    : relative >= 1
      ? `${formatNumber(relative)}x faster`
      : `${formatNumber(1 / relative)}x slower`;
  console.log(
    `| ${row.name} | ${output} | ${formatNs(row.medianNs)} | ${formatRate(row.opsPerSec)} | ${relativeText} |`
  );
}
console.log('');
console.log('## Notes');
console.log('');
console.log('- UUIDv4 outputs 36 hyphenated characters (122 random bits); NanoID outputs 21 characters (about 126 bits); RailID outputs 32 Base64URL characters (150 random bits).');
console.log('- `uuid` 14.0.3 uses Node.js `crypto.randomUUID` for the default v4 path on Node.');
console.log('- RailID OS-only is the explicitly degraded profile; the TPM row is the standard hardware profile.');
console.log('- The single-ID rows include the native N-API call, string allocation, and a forced ASCII copy.');
console.log('- The TPM row is hardware-bound and serialized by the TPM device.');
console.log('- Batch rows use a different API: one native call returns a contiguous encoded Buffer, then every ID boundary is read.');
console.log(`- Checksum: ${sink}`);

import { Worker } from 'node:worker_threads';
import os from 'node:os';

const WORKER_URL = new URL('./bench-worker.mjs', import.meta.url);
const TOTAL_IDS = 1_000_000;
const SAMPLES = 5;
const WARMUPS = 2;
const WORKER_COUNTS = [1, 2, 4, 6, 8, 12];
const MODES = [
  ['NanoID 21', 'nanoid'],
  ['UUIDv4 (uuid)', 'uuid'],
  ['RailID OS-only', 'railid'],
];

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

async function createPool(mode, count) {
  const workers = [];
  await Promise.all(
    Array.from({ length: count }, () => {
      const worker = new Worker(WORKER_URL, { workerData: { mode } });
      workers.push(worker);
      return new Promise((resolve, reject) => {
        worker.once('message', (message) => {
          if (message && message.ready) {
            resolve();
          } else {
            reject(new Error(`worker did not become ready: ${JSON.stringify(message)}`));
          }
        });
        worker.once('error', reject);
      });
    })
  );
  return workers;
}

async function runRound(workers, total) {
  const perWorker = Math.ceil(total / workers.length);
  const started = process.hrtime.bigint();
  const sinks = await Promise.all(
    workers.map(
      (worker) =>
        new Promise((resolve, reject) => {
          const onMessage = (message) => {
            worker.off('error', onError);
            if (message && message.ok) {
              resolve(message.sink);
            } else {
              reject(new Error(message && message.error ? message.error : 'worker failed'));
            }
          };
          const onError = (error) => {
            worker.off('message', onMessage);
            reject(error);
          };
          worker.once('message', onMessage);
          worker.once('error', onError);
          worker.postMessage({ count: perWorker });
        })
    )
  );
  const elapsed = Number(process.hrtime.bigint() - started);
  return { elapsedNs: elapsed, ids: perWorker * workers.length, sink: sinks.reduce((a, b) => a + b, 0) };
}

async function measure(mode, workers) {
  const pool = await createPool(mode, workers);
  try {
    for (let index = 0; index < WARMUPS; index++) {
      await runRound(pool, TOTAL_IDS);
    }
    const samples = [];
    let sink = 0;
    for (let index = 0; index < SAMPLES; index++) {
      const result = await runRound(pool, TOTAL_IDS);
      samples.push(result.elapsedNs / result.ids);
      sink += result.sink;
    }
    return { medianNs: median(samples), sink };
  } finally {
    await Promise.all(pool.map((worker) => worker.terminate()));
  }
}

function formatRate(ops) {
  if (ops >= 1e6) return `${(ops / 1e6).toFixed(2)} Mops/s`;
  if (ops >= 1e3) return `${(ops / 1e3).toFixed(2)} kops/s`;
  return `${ops.toFixed(2)} ops/s`;
}

const rows = [];
for (const [label, mode] of MODES) {
  for (const workers of WORKER_COUNTS) {
    const result = await measure(mode, workers);
    rows.push({
      mode: label,
      workers,
      nsPerId: result.medianNs,
      opsPerSec: 1e9 / result.medianNs,
      sink: result.sink,
    });
  }
}

const baseline = new Map(
  rows.filter((row) => row.workers === 1).map((row) => [row.mode, row.opsPerSec])
);
const cpus = os.cpus();
const cpu = cpus[0]?.model ?? 'unknown';

console.log('# NanoID / UUIDv4 / RailID OS-only multi-thread scaling');
console.log('');
console.log(`Generated: ${new Date().toISOString()}`);
console.log('');
console.log('| Item | Value |');
console.log('|---|---|');
console.log(`| OS | ${os.type()} ${os.release()} (${process.platform} ${process.arch}) |`);
console.log(`| CPU | ${cpu} |`);
console.log(`| Logical CPUs | ${cpus.length} |`);
console.log(`| Node.js | ${process.version} |`);
console.log(`| IDs per sample | ${TOTAL_IDS.toLocaleString()} |`);
console.log(`| Samples / warmups | ${SAMPLES} / ${WARMUPS} |`);
console.log('');
console.log('| Workload | Workers | Median/ID | Total throughput | Scaling vs 1 worker |');
console.log('|---|---:|---:|---:|---:|');
for (const row of rows) {
  const scaling = row.opsPerSec / baseline.get(row.mode);
  console.log(
    `| ${row.mode} | ${row.workers} | ${row.nsPerId.toFixed(2)} ns | ${formatRate(row.opsPerSec)} | ${scaling.toFixed(2)}x |`
  );
}
console.log('');
console.log('## Notes');
console.log('');
console.log('- Each worker uses an independent NanoID pool, UUID call path, or RailID OS-only native call.');
console.log('- Worker startup and teardown are excluded from the timed rounds.');
console.log('- Round time includes worker message dispatch and result collection.');
console.log('- RailID OS-only is the explicitly degraded profile; TPM is not included because the device is serialized.');
console.log(`- Checksum: ${rows.reduce((sum, row) => sum + row.sink, 0)}`);

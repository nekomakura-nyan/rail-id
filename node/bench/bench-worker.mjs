import { parentPort, workerData } from 'node:worker_threads';
import { createRequire } from 'node:module';
import { v4 as uuidv4 } from 'uuid';
import { nanoid } from 'nanoid';

const require = createRequire(import.meta.url);
const railid = workerData.mode === 'railid' ? require('../index.js') : null;

const scratch = Buffer.alloc(128);
let sink = 0;

function consume(value) {
  const length = scratch.write(value, 0, 'ascii');
  sink = (sink + scratch[0] + scratch[length - 1]) | 0;
}

// Bind the generator once, outside the timed loop. Selecting it per iteration
// makes the native call polymorphic and cost roughly 400 ns per ID on arm64.
let generate;
switch (workerData.mode) {
  case 'nanoid':
    generate = nanoid;
    break;
  case 'uuid':
    generate = uuidv4;
    break;
  case 'railid':
    generate = railid.generateOsOnly;
    break;
  default:
    throw new Error(`unknown mode: ${workerData.mode}`);
}

function run(count) {
  for (let index = 0; index < count; index++) {
    consume(generate());
  }
  return sink;
}

parentPort.on('message', (message) => {
  try {
    // Keep the hot loop out of the try block: wrapping it prevents V8 from
    // optimizing the native call on arm64.
    const value = run(message.count);
    parentPort.postMessage({ ok: true, sink: value });
  } catch (error) {
    parentPort.postMessage({ ok: false, error: error.message });
  }
});

parentPort.postMessage({ ready: true });

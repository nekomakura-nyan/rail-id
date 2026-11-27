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

function generate() {
  switch (workerData.mode) {
    case 'nanoid':
      return nanoid();
    case 'uuid':
      return uuidv4();
    case 'railid':
      return railid.generateOsOnly();
    default:
      throw new Error(`unknown mode: ${workerData.mode}`);
  }
}

parentPort.on('message', (message) => {
  try {
    for (let index = 0; index < message.count; index++) {
      consume(generate());
    }
    parentPort.postMessage({ ok: true, sink });
  } catch (error) {
    parentPort.postMessage({ ok: false, error: error.message });
  }
});

parentPort.postMessage({ ready: true });

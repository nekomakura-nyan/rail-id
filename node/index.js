'use strict';

const path = require('node:path');

function linuxLibc() {
  if (process.platform !== 'linux') {
    return '';
  }
  try {
    const report = process.report && process.report.getReport();
    const version = report && report.header && report.header.glibcVersionRuntime;
    return version ? 'gnu' : 'musl';
  } catch {
    return 'gnu';
  }
}

function suffix() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === 'win32') {
    return `win32-${arch}-msvc`;
  }
  if (platform === 'linux') {
    return `linux-${arch}-${linuxLibc()}`;
  }
  if (platform === 'darwin') {
    return `darwin-${arch}`;
  }
  return `${platform}-${arch}`;
}

function candidates() {
  const tag = suffix();
  if (process.platform === 'win32') {
    return [`railid.${tag}.node`];
  }
  if (process.platform === 'linux') {
    return [`railid-tpm.${tag}.node`, `railid.${tag}.node`];
  }
  return [`railid.${tag}.node`];
}

let binding = null;
let backend = null;
const loadErrors = [];

for (const file of candidates()) {
  try {
    binding = require(path.join(__dirname, file));
    backend = typeof binding.backend === 'function' ? binding.backend() : 'unknown';
    break;
  } catch (error) {
    loadErrors.push(`${file}: ${error.message}`);
  }
}

if (!binding) {
  const error = new Error(
    `No usable RailID native addon for ${process.platform}/${process.arch}. ` +
      `Tried: ${candidates().join(', ')}. ` +
      'Install a prebuilt package or run `npm run build` with Rust and the platform TPM libraries.'
  );
  error.cause = loadErrors.join('\n');
  throw error;
}

module.exports = {
  ...binding,
  // `backend` is the backend compiled into the loaded binary. A TPM build can
  // still return TpmUnavailable at runtime when no TPM device is present.
  backend: () => backend,
};

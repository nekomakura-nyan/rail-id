'use strict';

const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const nodeDir = path.resolve(__dirname, '..');
const targetDir = path.join(nodeDir, 'target', 'release');

function runCargo(args) {
  execFileSync('cargo', args, { cwd: nodeDir, stdio: 'inherit' });
}

function detectLibc() {
  if (process.platform !== 'linux') {
    return '';
  }
  try {
    const report = process.report && process.report.getReport();
    return report && report.header && report.header.glibcVersionRuntime ? 'gnu' : 'musl';
  } catch {
    return 'gnu';
  }
}

function outputName(kind) {
  const arch = process.arch;
  if (process.platform === 'win32') {
    return `railid.win32-${arch}-msvc.node`;
  }
  if (process.platform === 'linux') {
    const libc = detectLibc();
    return kind === 'tpm'
      ? `railid-tpm.linux-${arch}-${libc}.node`
      : `railid.linux-${arch}-${libc}.node`;
  }
  if (process.platform === 'darwin') {
    return `railid.darwin-${arch}.node`;
  }
  throw new Error(`unsupported platform: ${process.platform}/${process.arch}`);
}

function copyAddon(name, sourceDir = targetDir) {
  const source = path.join(
    sourceDir,
    process.platform === 'win32'
      ? 'railid_node.dll'
      : process.platform === 'darwin'
        ? 'librailid_node.dylib'
        : 'librailid_node.so'
  );
  if (!fs.existsSync(source)) {
    throw new Error(`native addon not found: ${source}`);
  }
  const destination = path.join(nodeDir, name);
  fs.copyFileSync(source, destination);
  console.log(`wrote ${path.relative(process.cwd(), destination)}`);
}

const osOnly = process.argv.includes('--os-only');

if (process.platform === 'win32') {
  runCargo(['build', '--release', '--features', 'tpm']);
  copyAddon(outputName('tpm'));
} else if (process.platform === 'linux') {
  if (!osOnly) {
    runCargo(['build', '--release', '--features', 'tpm']);
    copyAddon(outputName('tpm'));
  }
  runCargo(['build', '--release', '--no-default-features']);
  copyAddon(outputName('os'));
} else if (process.platform === 'darwin') {
  // macOS ships both architectures. Build both from the same host; the macOS
  // SDK is universal. Install the Rust targets first:
  //   rustup target add x86_64-apple-darwin aarch64-apple-darwin
  for (const [target, name] of [
    ['x86_64-apple-darwin', 'railid.darwin-x64.node'],
    ['aarch64-apple-darwin', 'railid.darwin-arm64.node'],
  ]) {
    runCargo(['build', '--release', '--no-default-features', '--target', target]);
    copyAddon(name, path.join(nodeDir, 'target', target, 'release'));
  }
} else {
  throw new Error(`unsupported platform: ${process.platform}/${process.arch}`);
}

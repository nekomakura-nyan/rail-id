'use strict';

// Build the N-API addon for the host or for an explicit cross target.
//
// Target names use the same platform tags as the published `.node` files:
//
//   node scripts/build.js                                   # host default
//   node scripts/build.js --target linux-arm64-musl --os-only
//   node scripts/build.js --target darwin-arm64
//
// Run `node scripts/build.js --list` to see every supported target.

const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const nodeDir = path.resolve(__dirname, '..');

// platform/arch/abi are the Node.js `process.platform`/`process.arch` values
// and the libc tag used in the artifact name.
const TARGETS = {
  'linux-x64-gnu': {
    triple: 'x86_64-unknown-linux-gnu',
    platform: 'linux',
    arch: 'x64',
    libc: 'gnu',
  },
  'linux-x64-musl': {
    triple: 'x86_64-unknown-linux-musl',
    platform: 'linux',
    arch: 'x64',
    libc: 'musl',
  },
  'linux-arm64-gnu': {
    triple: 'aarch64-unknown-linux-gnu',
    platform: 'linux',
    arch: 'arm64',
    libc: 'gnu',
  },
  'linux-arm64-musl': {
    triple: 'aarch64-unknown-linux-musl',
    platform: 'linux',
    arch: 'arm64',
    libc: 'musl',
  },
  'linux-arm-gnu': {
    triple: 'armv7-unknown-linux-gnueabihf',
    platform: 'linux',
    arch: 'arm',
    libc: 'gnu',
  },
  'linux-riscv64-gnu': {
    triple: 'riscv64gc-unknown-linux-gnu',
    platform: 'linux',
    arch: 'riscv64',
    libc: 'gnu',
  },
  'win32-x64-msvc': {
    triple: 'x86_64-pc-windows-msvc',
    platform: 'win32',
    arch: 'x64',
  },
  'win32-arm64-msvc': {
    triple: 'aarch64-pc-windows-msvc',
    platform: 'win32',
    arch: 'arm64',
  },
  'darwin-x64': {
    triple: 'x86_64-apple-darwin',
    platform: 'darwin',
    arch: 'x64',
  },
  'darwin-arm64': {
    triple: 'aarch64-apple-darwin',
    platform: 'darwin',
    arch: 'arm64',
  },
};

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

function hostTargetName() {
  if (process.platform === 'win32') {
    return `win32-${process.arch}-msvc`;
  }
  if (process.platform === 'linux') {
    return `linux-${process.arch}-${detectLibc()}`;
  }
  if (process.platform === 'darwin') {
    return `darwin-${process.arch}`;
  }
  return null;
}

function artifactName(target, targetName, kind) {
  // Windows has a TPM backend built in and ships a single binary; Linux keeps
  // the TPM and OS-only addons separate so the OS-only one has no libtss2
  // runtime dependency.
  if (kind === 'tpm' && target.platform === 'linux') {
    return `railid-tpm.${targetName}.node`;
  }
  return `railid.${targetName}.node`;
}

function libraryName(target) {
  if (target.platform === 'win32') {
    return 'railid_node.dll';
  }
  if (target.platform === 'darwin') {
    return 'librailid_node.dylib';
  }
  return 'librailid_node.so';
}

function cargoBuild(target, targetName, kind) {
  const triple = TARGETS[targetName].triple;
  const native = triple === hostTriple();
  const args = ['build', '--release'];
  if (!native) {
    args.push('--target', triple);
  }
  args.push(...(kind === 'tpm' ? ['--features', 'tpm'] : ['--no-default-features']));
  console.log(`cargo ${args.join(' ')}`);
  execFileSync('cargo', args, { cwd: nodeDir, stdio: 'inherit' });

  const releaseDir = native
    ? path.join(nodeDir, 'target', 'release')
    : path.join(nodeDir, 'target', triple, 'release');
  copyAddon(target, artifactName(target, targetName, kind), releaseDir);
}

function hostTriple() {
  // `rustc -vV` is the only reliable way to learn the host triple without
  // pulling in a dependency.
  const output = execFileSync('rustc', ['-vV'], { encoding: 'utf8' });
  const match = /^host:\s*(\S+)$/m.exec(output);
  if (!match) {
    throw new Error('could not determine the host triple from `rustc -vV`');
  }
  return match[1];
}

function copyAddon(target, name, sourceDir = path.join(nodeDir, 'target', 'release')) {
  const source = path.join(sourceDir, libraryName(target));
  if (!fs.existsSync(source)) {
    throw new Error(`native addon not found: ${source}`);
  }
  const destination = path.join(nodeDir, name);
  fs.copyFileSync(source, destination);
  console.log(`wrote ${path.relative(process.cwd(), destination)}`);
}

function parseArgs(argv) {
  const options = { target: null, osOnly: false, tpm: false, list: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--target') {
      options.target = argv[index + 1];
      index += 1;
      if (!options.target) {
        throw new Error('--target needs a value');
      }
    } else if (arg.startsWith('--target=')) {
      options.target = arg.slice('--target='.length);
    } else if (arg === '--os-only') {
      options.osOnly = true;
    } else if (arg === '--tpm') {
      options.tpm = true;
    } else if (arg === '--list') {
      options.list = true;
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (options.osOnly && options.tpm) {
    throw new Error('--os-only and --tpm are mutually exclusive');
  }
  return options;
}

function kindsFor(targetName, options) {
  if (options.tpm) {
    return ['tpm'];
  }
  if (options.osOnly) {
    return ['os'];
  }
  const target = TARGETS[targetName];
  // TPM is built automatically only for native x64 Linux (glibc) and for any
  // Windows target, where TBS is part of the OS. Cross and musl Linux builds
  // are OS-only unless `--tpm` is passed explicitly.
  const native = target.triple === hostTriple();
  if (target.platform === 'win32') {
    return ['tpm'];
  }
  if (native && target.platform === 'linux' && target.arch === 'x64' && target.libc === 'gnu') {
    return ['tpm', 'os'];
  }
  return ['os'];
}

function main() {
  const options = parseArgs(process.argv.slice(2));

  if (options.list) {
    for (const name of Object.keys(TARGETS)) {
      console.log(name);
    }
    return;
  }

  const host = hostTargetName();
  const requested = options.target || host;
  if (!requested || !TARGETS[requested]) {
    throw new Error(
      `unsupported target: ${requested || `${process.platform}/${process.arch}`}\n` +
        `known targets: ${Object.keys(TARGETS).join(', ')}`
    );
  }

  const kinds = kindsFor(requested, options);
  for (const kind of kinds) {
    cargoBuild(TARGETS[requested], requested, kind);
  }

  // A bare `npm run build` on macOS keeps producing both slices so the host
  // tarball is universal, matching the published package.
  if (!options.target && !options.osOnly && !options.tpm && process.platform === 'darwin') {
    for (const name of ['darwin-x64', 'darwin-arm64']) {
      if (name === host) {
        continue;
      }
      cargoBuild(TARGETS[name], name, 'os');
    }
  }
}

try {
  main();
} catch (error) {
  console.error(`build failed: ${error.message}`);
  process.exitCode = 1;
}

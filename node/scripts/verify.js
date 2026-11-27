'use strict';

// Verify that prebuilt `.node` binaries match the platform tag in their file
// name. This catches cross-compilation mistakes on targets that cannot be
// executed on the build host (for example linux-riscv64-gnu).
//
//   node scripts/verify.js                 # check every *.node in node/
//   node scripts/verify.js --target linux-arm64-musl

const fs = require('node:fs');
const path = require('node:path');

const nodeDir = path.resolve(__dirname, '..');

const MACHINES = {
  elf: {
    40: ['arm'],
    62: ['x64'],
    183: ['arm64'],
    243: ['riscv64'],
  },
  pe: {
    0x014c: ['ia32'],
    0x8664: ['x64'],
    0xaa64: ['arm64'],
  },
  macho: {
    0x00000007: ['x64'],
    0x01000007: ['x64'],
    0x0000000c: ['arm64'],
    0x0100000c: ['arm64'],
  },
};

function parseTag(name) {
  const match = /^railid(?:-tpm)?\.(.+)\.node$/.exec(name);
  if (!match) {
    throw new Error(`unexpected addon name: ${name}`);
  }
  const tag = match[1];
  const parts = tag.split('-');
  const platform = parts[0];
  const arch = parts[1];
  return { name, tag, platform, arch, tpm: name.startsWith('railid-tpm.') };
}

function describe(buffer, file) {
  if (buffer.length < 64) {
    throw new Error(`${file}: file is too small to be a shared library`);
  }

  if (buffer[0] === 0x7f && buffer.toString('latin1', 1, 4) === 'ELF') {
    const is64 = buffer[4] === 2;
    const machine = buffer.readUInt16LE(18);
    return { kind: 'elf', bits: is64 ? 64 : 32, arch: (MACHINES.elf[machine] || ['unknown'])[0] };
  }

  if (buffer[0] === 0x4d && buffer[1] === 0x5a) {
    const peOffset = buffer.readUInt32LE(0x3c);
    if (buffer.toString('latin1', peOffset, peOffset + 4) !== 'PE\u0000\u0000') {
      throw new Error(`${file}: invalid PE signature`);
    }
    const machine = buffer.readUInt16LE(peOffset + 4);
    const magic = buffer.readUInt16LE(peOffset + 24);
    return {
      kind: 'pe',
      bits: magic === 0x20b ? 64 : 32,
      arch: (MACHINES.pe[machine] || ['unknown'])[0],
    };
  }

  const magic = buffer.readUInt32LE(0);
  if (magic === 0xfeedfacf || magic === 0xcffaedfe) {
    const cputype = buffer.readUInt32LE(4);
    return {
      kind: 'macho',
      bits: 64,
      arch: (MACHINES.macho[cputype] || ['unknown'])[0],
    };
  }

  throw new Error(`${file}: unrecognized binary format`);
}

function check(file, buffer, expected) {
  const actual = describe(buffer, file);
  const problems = [];

  const expectedKind =
    expected.platform === 'win32' ? 'pe' : expected.platform === 'darwin' ? 'macho' : 'elf';
  if (actual.kind !== expectedKind) {
    problems.push(`format ${actual.kind} != ${expectedKind}`);
  }
  if (actual.arch !== expected.arch) {
    problems.push(`arch ${actual.arch} != ${expected.arch}`);
  }
  const expectedBits = expected.arch === 'arm' || expected.arch === 'ia32' ? 32 : 64;
  if (actual.bits !== expectedBits) {
    problems.push(`${actual.bits}-bit != ${expectedBits}-bit`);
  }
  if (!buffer.includes('napi_register_module_v1')) {
    problems.push('missing napi_register_module_v1 symbol string');
  }
  if (buffer.length < 16 * 1024) {
    problems.push(`suspiciously small (${buffer.length} bytes)`);
  }

  return { ...actual, tag: expected.tag, tpm: expected.tpm, problems };
}

function parseArgs(argv) {
  let target = null;
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--target') {
      target = argv[index + 1];
      index += 1;
    } else if (arg.startsWith('--target=')) {
      target = arg.slice('--target='.length);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return { target };
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const files = fs
    .readdirSync(nodeDir)
    .filter((name) => name.endsWith('.node'))
    .filter((name) => (options.target ? name === `railid.${options.target}.node` || name === `railid-tpm.${options.target}.node` : true))
    .sort();

  if (files.length === 0) {
    throw new Error(
      options.target ? `no addon for target ${options.target}` : 'no .node files found'
    );
  }

  let failures = 0;
  for (const file of files) {
    const expected = parseTag(file);
    const buffer = fs.readFileSync(path.join(nodeDir, file));
    try {
      const result = check(file, buffer, expected);
      if (result.problems.length > 0) {
        failures += 1;
        console.error(`FAIL ${file}: ${result.problems.join(', ')}`);
      } else {
        console.log(
          `ok   ${file} ${result.kind} ${result.arch} ${result.bits}-bit ${buffer.length} bytes${
            result.tpm ? ' (tpm)' : ''
          }`
        );
      }
    } catch (error) {
      failures += 1;
      console.error(`FAIL ${file}: ${error.message}`);
    }
  }

  if (failures > 0) {
    throw new Error(`${failures} of ${files.length} addon(s) failed verification`);
  }
  console.log(`verified ${files.length} addon(s)`);
}

try {
  main();
} catch (error) {
  console.error(`verify failed: ${error.message}`);
  process.exitCode = 1;
}

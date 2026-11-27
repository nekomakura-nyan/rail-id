'use strict';

// TPM-less entry point.
//
// The default entry point keeps the fail-closed TPM hybrid profile: `generate()`
// throws when no TPM is available. This module is the explicit opt-in for hosts
// that never have a TPM. Every generator here uses the degraded OS-only
// profile, and the choice is visible at the import site.
const core = require('./index.js');

module.exports = {
  ...core,
  generate: core.generateOsOnly,
  generateBuffer: core.generateOsOnlyBuffer,
  // The `osOnly` argument is accepted for signature compatibility and always
  // resolves to the OS-only profile.
  generateMany: (count) => core.generateMany(count, true),
  generateManyEncoded: (count) => core.generateManyEncoded(count, true),
  generateManyAsync: (count) => core.generateManyAsync(count, true),
  generateManyEncodedAsync: (count) => core.generateManyEncodedAsync(count, true),
  // The loaded binary may still contain the TPM backend, but this module never
  // calls it.
  backend: () => 'os-only',
};

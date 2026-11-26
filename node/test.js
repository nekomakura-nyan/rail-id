'use strict';

const assert = require('node:assert');
const railid = require('./index');

async function main() {
  assert.match(railid.version(), /^\d+\.\d+\.\d+/);
  assert.ok(['tpm', 'os-only'].includes(railid.backend()));

  const osId = railid.generateOsOnly();
  assert.match(osId, /^[A-Za-z0-9_-]{32}$/);
  const decoded = railid.decode(osId);
  assert.equal(decoded.encoded, osId);
  assert.ok(decoded.timestampMs > 0);
  assert.match(decoded.randomHex, /^[0-9a-f]{38}$/);

  assert.equal(railid.generateManyEncoded(0, true).length, 0);
  assert.deepEqual(railid.generateMany(0, true), []);

  const many = railid.generateMany(8, true);
  assert.equal(many.length, 8);
  for (const encoded of many) {
    assert.match(encoded, /^[A-Za-z0-9_-]{32}$/);
  }

  const batch = railid.generateManyEncoded(8, true);
  assert.ok(Buffer.isBuffer(batch));
  assert.equal(batch.length, 8 * 32);
  for (let index = 0; index < 8; index++) {
    const encoded = batch.toString('ascii', index * 32, (index + 1) * 32);
    assert.equal(railid.decode(encoded).encoded, encoded);
  }

  const asyncMany = await railid.generateManyAsync(8, true);
  assert.equal(asyncMany.length, 8);
  for (const encoded of asyncMany) {
    assert.match(encoded, /^[A-Za-z0-9_-]{32}$/);
  }

  const asyncBatch = await railid.generateManyEncodedAsync(8, true);
  assert.ok(Buffer.isBuffer(asyncBatch));
  assert.equal(asyncBatch.length, 8 * 32);

  assert.throws(() => railid.decode('not-a-railid'), /Error/);
  assert.throws(
    () => railid.generateManyEncoded(0xffffffff, true),
    /exceeds binding limit/
  );
  assert.throws(
    () => railid.generateMany(0xffffffff, true),
    /exceeds binding limit/
  );

  let tpmNote = 'TPM profile not exercised';
  try {
    const tpmId = railid.generate();
    assert.match(tpmId, /^[A-Za-z0-9_-]{32}$/);
    const tpmBatch = railid.generateManyEncoded(4, false);
    assert.equal(tpmBatch.length, 4 * 32);
    const tpmMany = railid.generateMany(4);
    assert.equal(tpmMany.length, 4);
    const tpmAsync = await railid.generateManyAsync(2);
    assert.equal(tpmAsync.length, 2);
    tpmNote = 'TPM profile OK';
  } catch (error) {
    if (process.env.RAILID_REQUIRE_TPM === '1') {
      throw error;
    }
    tpmNote = `TPM profile unavailable: ${error.message}`;
  }

  console.log(`railid ${railid.version()} (${railid.backend()} backend)`);
  console.log(`OS-only: ${osId}`);
  console.log(`encoded batch: ${batch.length} bytes`);
  console.log(`async batch: ${asyncBatch.length} bytes`);
  console.log(tpmNote);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

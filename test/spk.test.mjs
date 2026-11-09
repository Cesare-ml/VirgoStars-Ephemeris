import test from 'node:test';
import assert from 'node:assert/strict';
import { inspectSpk, chebyshevValueAndDerivative, stateAtTdbSeconds, SpkError } from '../src/index.mjs';

/** Build a tiny ORIGINAL SYNTHETIC binary DAF/SPK file, not a NASA kernel. */
function fakeSpk({ bigEndian = false, segments = undefined } = {}) {
  const specs = segments ?? [
    { target: 3, center: 0, name: 'EMB', cx: [5, 0] },
    { target: 399, center: 3, name: 'Earth', cx: [1, 0] },
    { target: 10, center: 0, name: 'Sun', cx: [10, 2] },
  ];
  const b = Buffer.alloc(4 * 1024);
  const writeI = (v, off) => bigEndian ? b.writeInt32BE(v, off) : b.writeInt32LE(v, off);
  const writeD = (v, off) => bigEndian ? b.writeDoubleBE(v, off) : b.writeDoubleLE(v, off);
  b.write('DAF/SPK ', 0, 'ascii');
  writeI(2, 8); writeI(6, 12);
  b.write('SYNTHETIC TEST ONLY', 16, 'ascii');
  writeI(2, 76); writeI(2, 80); writeI(600, 84);
  b.write(bigEndian ? 'BIG-IEEE' : 'LTL-IEEE', 88, 'ascii');
  // Record 2: summary. Record 3: name. Record 4: double array data.
  writeD(0, 1024); writeD(0, 1032); writeD(specs.length, 1040);
  let word = 385;
  for (const [i, spec] of specs.entries()) {
    const offset = 1024 + 24 + i * 40;
    writeD(-10, offset); writeD(10, offset + 8);
    [spec.target, spec.center, 1, 2, word, word + 11].forEach((v, j) => writeI(v, offset + 16 + 4 * j));
    b.write(spec.name, 2048 + i * 40, 'ascii');
    // MID, RADIUS, coefficients X(2), Y(2), Z(2), directory(4)
    [0, 10, ...spec.cx, 0, 0, 0, 0, -10, 20, 8, 1]
      .forEach((v, j) => writeD(v, (word + j - 1) * 8));
    word += 12;
  }
  return b;
}

test('reads little and big endian DAF/SPK segment summaries', () => {
  for (const bigEndian of [false, true]) {
    const data = inspectSpk(fakeSpk({ bigEndian }));
    assert.equal(data.format, bigEndian ? 'BIG-IEEE' : 'LTL-IEEE');
    assert.deepEqual(data.segments.map(s => s.name), ['EMB', 'Earth', 'Sun']);
    assert.deepEqual(data.segments.map(s => s.target), [3, 399, 10]);
  }
});

test('Chebyshev evaluation includes analytic velocity', () => {
  assert.deepEqual(chebyshevValueAndDerivative([10, 2], 0.5), { value: 11, derivative: 2 });
  assert.deepEqual(chebyshevValueAndDerivative([1, 0, 3], 0), { value: -2, derivative: 0 });
});

test('resolves Sun relative to Earth via Earth-Moon barycenter chain', () => {
  for (const bigEndian of [false, true]) {
    const state = stateAtTdbSeconds(fakeSpk({ bigEndian }), 10, 399, 5);
    assert.deepEqual(state.positionKm, [5, 0, 0]);
    assert.deepEqual(state.velocityKmPerSec, [0.2, 0, 0]);
    assert.deepEqual(state.sourceSegments, ['Sun', 'Earth', 'EMB']);
    assert.equal(state.corrections, 'GEOMETRIC');
  }
});

test('handles inclusive endpoints of type-2 record', () => {
  assert.deepEqual(stateAtTdbSeconds(fakeSpk(), 10, 0, -10).positionKm, [8, 0, 0]);
  assert.deepEqual(stateAtTdbSeconds(fakeSpk(), 10, 0, 10).positionKm, [12, 0, 0]);
});

test('rejects unsupported intervals, bodies and malformed files', () => {
  const spk = fakeSpk();
  assert.throws(() => stateAtTdbSeconds(spk, 10, 399, 11), e => e.code === 'MISSING_CENTER_CHAIN');
  assert.throws(() => stateAtTdbSeconds(spk, 301, 0, 5), e => e.code === 'MISSING_CENTER_CHAIN');
  assert.throws(() => inspectSpk(Buffer.alloc(1024)), SpkError);
  const copy = Buffer.from(spk);
  copy.writeInt32LE(0, 76);
  assert.throws(() => inspectSpk(copy), SpkError);
});

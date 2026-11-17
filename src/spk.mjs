// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

/**
 * Independent, read-only SPK/DAF Type 2 reader.
 * Specification: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html
 *                https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html
 * Supports ICRF/J2000 (NAIF frame ID 1) geometric km and km/s only.
 * No Swiss Ephemeris source or runtime is used.
 */
export class SpkError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'SpkError';
    this.code = code;
  }
}
const invalid = (code, message) => { throw new SpkError(code, message); };

const RECORD_BYTES = 1024;
const MAX_SUMMARY_RECORDS = 100000;
const MAX_CHAIN_DEPTH = 64;
const isInt = x => Number.isSafeInteger(x);

/** Read the DAF file record and segment summary list from a Buffer. */
export function inspectSpk(buffer) {
  if (!Buffer.isBuffer(buffer) || buffer.length < RECORD_BYTES)
    invalid('INVALID_SPK', 'SPK input must be a Buffer containing at least 1024 bytes');
  if (buffer.toString('ascii', 0, 8) !== 'DAF/SPK ')
    invalid('INVALID_SPK', 'Not a DAF/SPK file');
  const format = buffer.toString('ascii', 88, 96);
  if (format !== 'LTL-IEEE' && format !== 'BIG-IEEE')
    invalid('UNSUPPORTED_FORMAT', `Unsupported DAF binary format: ${format}`);
  const le = format === 'LTL-IEEE';
  const int = offset => le ? buffer.readInt32LE(offset) : buffer.readInt32BE(offset);
  const dbl = offset => le ? buffer.readDoubleLE(offset) : buffer.readDoubleBE(offset);
  const nd = int(8), ni = int(12);
  if (nd !== 2 || ni !== 6) invalid('UNSUPPORTED_SUMMARY', `Expected SPK ND=2 NI=6, got ${nd}/${ni}`);
  const fward = int(76), bward = int(80), free = int(84);
  const lastRecord = Math.floor(buffer.length / RECORD_BYTES);
  if (!isInt(fward) || fward < 2 || fward > lastRecord || bward < fward || bward > lastRecord || free < 1)
    invalid('INVALID_SPK', 'Invalid DAF summary pointers');
  const segments = [];
  const visited = new Set();
  let record = fward;
  while (record !== 0) {
    if (visited.size >= MAX_SUMMARY_RECORDS || visited.has(record) || record < 2 || record >= lastRecord)
      invalid('INVALID_SPK', 'DAF summary chain cycle or invalid pointer');
    visited.add(record);
    const offset = (record - 1) * RECORD_BYTES;
    const next = dbl(offset), previous = dbl(offset + 8), count = dbl(offset + 16);
    if (!isInt(next) || !isInt(previous) || !isInt(count) || count < 0 || count > 25)
      invalid('INVALID_SPK', 'Malformed DAF summary record header');
    if ((visited.size === 1 && previous !== 0) || (next && (next < 2 || next > lastRecord)))
      invalid('INVALID_SPK', 'Malformed DAF linked summary records');
    for (let i = 0; i < count; i++) {
      const start = offset + 24 + i * 40;
      const startEt = dbl(start), endEt = dbl(start + 8);
      const descriptor = [];
      for (let j = 0; j < 6; j++) descriptor.push(int(start + 16 + j * 4));
      const [target, center, frame, type, startAddress, endAddress] = descriptor;
      if (!Number.isFinite(startEt) || !Number.isFinite(endEt) || startEt > endEt ||
          startAddress < 1 || endAddress < startAddress || 8 * endAddress > buffer.length)
        invalid('INVALID_SPK', 'Invalid segment descriptor or addresses');
      const name = buffer.toString('ascii', record * RECORD_BYTES + i * 40,
        record * RECORD_BYTES + (i + 1) * 40).replace(/\0/g, '').trim();
      segments.push(Object.freeze({
        name, target, center, frame, type,
        startEt, endEt, startAddress, endAddress,
      }));
    }
    record = next;
  }
  return Object.freeze({
    format, internalName: buffer.toString('ascii', 16, 76).trim(),
    segments: Object.freeze(segments),
  });
}

/** Calculate a Chebyshev series and its derivative with respect to x in [-1,1]. */
export function chebyshevValueAndDerivative(coefficients, x) {
  if (!Array.isArray(coefficients) || coefficients.length === 0 ||
      !Number.isFinite(x) || x < -1 - 1e-12 || x > 1 + 1e-12 ||
      coefficients.some(c => !Number.isFinite(c)))
    invalid('INVALID_CHEBYSHEV_INPUT', 'Invalid Chebyshev coefficients or normalized epoch');
  let value = coefficients[0], derivative = 0;
  let t0 = 1, t1 = x, d0 = 0, d1 = 1;
  if (coefficients.length > 1) { value += coefficients[1] * x; derivative += coefficients[1]; }
  for (let i = 2; i < coefficients.length; i++) {
    const t2 = 2 * x * t1 - t0;
    const d2 = 2 * t1 + 2 * x * d1 - d0;
    value += coefficients[i] * t2;
    derivative += coefficients[i] * d2;
    t0 = t1; t1 = t2; d0 = d1; d1 = d2;
  }
  return Object.freeze({ value, derivative });
}

/** SPK Type 2: Chebyshev position (km), derivative velocity (km/s). */
export function evaluateType2(buffer, segment, et, format) {
  if (segment.type !== 2) invalid('UNSUPPORTED_SPK_TYPE', `SPK type ${segment.type} is not yet implemented`);
  if (!Number.isFinite(et) || et < segment.startEt || et > segment.endEt)
    invalid('OUT_OF_KERNEL_RANGE', 'Epoch is outside segment descriptor interval');
  const littleEndian = format === 'LTL-IEEE';
  const word = address => {
    if (!isInt(address) || address < segment.startAddress || address > segment.endAddress)
      invalid('INVALID_SPK', 'Word address outside SPK segment');
    const pos = (address - 1) * 8;
    return littleEndian ? buffer.readDoubleLE(pos) : buffer.readDoubleBE(pos);
  };
  const { startAddress: a, endAddress: b } = segment;
  const init = word(b - 3), intlen = word(b - 2);
  const rsize = word(b - 1), n = word(b);
  if (!Number.isFinite(init) || !Number.isFinite(intlen) || intlen <= 0 ||
      !isInt(rsize) || rsize < 5 || (rsize - 2) % 3 !== 0 ||
      !isInt(n) || n < 1 || (b - a + 1 !== rsize * n + 4))
    invalid('INVALID_SPK', 'Malformed type-2 segment directory');
  const index = Math.min(n - 1, Math.floor((et - init) / intlen));
  if (index < 0 || et > init + n * intlen + 1e-7)
    invalid('OUT_OF_KERNEL_RANGE', 'Epoch is outside type-2 record coverage');
  const base = a + index * rsize;
  const midpoint = word(base), radius = word(base + 1);
  if (!Number.isFinite(midpoint) || !Number.isFinite(radius) || radius <= 0)
    invalid('INVALID_SPK', 'Malformed type-2 Chebyshev record');
  const x = (et - midpoint) / radius;
  if (x < -1 - 1e-10 || x > 1 + 1e-10)
    invalid('INVALID_SPK', 'Record interval and directory disagree');
  const length = (rsize - 2) / 3;
  const positionKm = [], velocityKmPerSec = [];
  for (let axis = 0; axis < 3; axis++) {
    const coeff = Array.from({ length }, (_, j) => word(base + 2 + axis * length + j));
    const value = chebyshevValueAndDerivative(coeff, Math.max(-1, Math.min(1, x)));
    positionKm.push(value.value);
    velocityKmPerSec.push(value.derivative / radius);
  }
  return Object.freeze({ positionKm: Object.freeze(positionKm), velocityKmPerSec: Object.freeze(velocityKmPerSec) });
}

import { evaluateType3 } from './spk3.mjs';

/**
 * Resolve target minus center using segment chains leading to the same root.
 * Input et is TDB seconds from J2000 (JD 2451545.0), NOT UTC or TT seconds.
 * Currently supports geometric J2000 states from SPK type 2 only.
 */
export function stateAtTdbSeconds(buffer, target, center, et) {
  if (!Number.isInteger(target) || !Number.isInteger(center) || !Number.isFinite(et))
    invalid('INVALID_ARGUMENT', 'Target, center and finite TDB seconds are required');
  const spk = inspectSpk(buffer);
  if (target === center) return Object.freeze({
    target, center, etTdbSeconds: et, frame: 'J2000', corrections: 'GEOMETRIC',
    positionKm: Object.freeze([0, 0, 0]), velocityKmPerSec: Object.freeze([0, 0, 0]),
    sourceSegments: Object.freeze([]),
  });
  // NAIF DAF/SPK priority is file order: the last covering segment for a
  // target wins, irrespective of frame or type. Never fall back to an older
  // J2000 segment when the highest-priority segment is unsupported.
  const active = new Map();
  for (const s of spk.segments) {
    if (s.startEt <= et && et <= s.endEt) active.set(s.target, s);
  }
  const pathToRoot = (body) => {
    const path = [], visited = new Set();
    for (let i = 0; i < MAX_CHAIN_DEPTH; i++) {
      if (body === 0) return path;
      if (visited.has(body)) invalid('INVALID_SPK', 'Segment center-chain cycle detected');
      visited.add(body);
      const s = active.get(body);
      if (!s) invalid('MISSING_CENTER_CHAIN', `No covering segment for body ${body} at requested epoch`);
      if (s.frame !== 1)
        invalid('UNSUPPORTED_SPK_FRAME', `Highest-priority covering segment for body ${body} uses unsupported frame ${s.frame}`);
      path.push(s);
      body = s.center;
    }
    invalid('INVALID_SPK', 'Segment center chain exceeds safety bound');
  };
  const a = pathToRoot(target), b = pathToRoot(center);
  const pos = [0, 0, 0], vel = [0, 0, 0];
  for (const [path, sign] of [[a, 1], [b, -1]]) {
    for (const s of path) {
      const state = s.type === 2 ? evaluateType2(buffer, s, et, spk.format) :
        s.type === 3 ? evaluateType3(buffer, s, et, spk.format) :
        invalid('UNSUPPORTED_SPK_TYPE', `SPK type ${s.type} is not implemented`);
      for (let k = 0; k < 3; k++) {
        pos[k] += sign * state.positionKm[k];
        vel[k] += sign * state.velocityKmPerSec[k];
      }
    }
  }
  return Object.freeze({
    target, center, etTdbSeconds: et, frame: 'J2000', corrections: 'GEOMETRIC',
    positionKm: Object.freeze(pos), velocityKmPerSec: Object.freeze(vel),
    sourceSegments: Object.freeze([...new Set([...a, ...b].map(s => s.name))]),
  });
}

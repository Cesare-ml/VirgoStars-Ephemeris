import { openSync, readSync, closeSync, fstatSync } from 'node:fs';

// Classic JPL DE441 binary, not NAIF SPK or Astrodienst's SE1 format.
// Coordinates are geometric, equatorial ICRF, in km and km/day; dates are JD TDB.
const SERIES = Object.freeze({
  mercury: 0, venus: 1, earthMoonBarycenter: 2, marsBarycenter: 3,
  jupiterBarycenter: 4, saturnBarycenter: 5, uranusBarycenter: 6,
  neptuneBarycenter: 7, plutoBarycenter: 8, moonGeocentric: 9, sun: 10,
});

function readExact(fd, length, offset) {
  const data = Buffer.alloc(length);
  let done = 0;
  while (done < length) {
    const count = readSync(fd, data, done, length - done, offset + done);
    if (count === 0) throw new Error('Truncated JPL ephemeris file');
    done += count;
  }
  return data;
}

function combine(a, b, factor) {
  return {
    position: a.position.map((v, i) => v + factor * b.position[i]),
    velocity: a.velocity.map((v, i) => v + factor * b.velocity[i]),
  };
}

/** Open a classic, little-endian DE441 file. Close the returned reader when finished. */
export function openDe441(path) {
  const fd = openSync(path, 'r');
  try {
    const header = readExact(fd, 2856, 0);
    if (!header.toString('ascii', 0, 84).startsWith('JPL Planetary Ephemeris DE441') || header.readInt32LE(2840) !== 441) {
      throw new Error('Expected a little-endian classic JPL DE441 ephemeris');
    }
    const start = header.readDoubleLE(2652);
    const end = header.readDoubleLE(2660);
    const step = header.readDoubleLE(2668);
    const earthMoonMassRatio = header.readDoubleLE(2688);
    const pointers = Array.from({ length: 13 }, (_, i) => {
      const offset = i === 12 ? 2844 : 2696 + i * 12;
      return [0, 1, 2].map(j => header.readInt32LE(offset + j * 4));
    });
    if (![start, end, step, earthMoonMassRatio].every(Number.isFinite) || end <= start || step <= 0 || earthMoonMassRatio <= 0) {
      throw new Error('Invalid JPL ephemeris header');
    }
    let words = 2;
    pointers.forEach(([first, count, granules], i) => {
      if (first === 0 && count === 0 && granules === 0) return;
      if (first < 3 || count <= 0 || granules <= 0 || count > 100 || granules > 100) throw new Error('Invalid JPL coefficient pointer');
      words = Math.max(words, first - 1 + count * granules * (i === 11 ? 2 : 3));
    });
    const recordSize = words * 8;
    const recordCount = (end - start) / step;
    if (!Number.isSafeInteger(recordCount) || recordSize < 2856 || fstatSync(fd).size !== (recordCount + 2) * recordSize) {
      throw new Error('Inconsistent JPL record size or file length');
    }
    let closed = false;
    function series(index, jd) {
      const [first, count, granules] = pointers[index];
      if (!first) throw new Error('Requested series is not present in this file');
      const recordIndex = Math.min(Math.floor((jd - start) / step), recordCount - 1);
      const record = readExact(fd, recordSize, (recordIndex + 2) * recordSize);
      const recordStart = record.readDoubleLE(0);
      const recordEnd = record.readDoubleLE(8);
      if (recordStart !== start + recordIndex * step || recordEnd !== recordStart + step) throw new Error('Unexpected JPL coefficient record dates');
      const granuleWidth = step / granules;
      const granule = Math.min(Math.floor((jd - recordStart) / granuleWidth), granules - 1);
      const x = 2 * (jd - (recordStart + granule * granuleWidth)) / granuleWidth - 1;
      const position = [], velocity = [];
      for (let axis = 0; axis < 3; axis++) {
        const offset = (first - 1 + (granule * 3 + axis) * count) * 8;
        let t0 = 1, t1 = x, d0 = 0, d1 = 1;
        let p = record.readDoubleLE(offset), v = 0;
        if (count > 1) { const c = record.readDoubleLE(offset + 8); p += c * t1; v += c; }
        for (let k = 2; k < count; k++) {
          const t = 2 * x * t1 - t0;
          const d = 2 * t1 + 2 * x * d1 - d0;
          const c = record.readDoubleLE(offset + k * 8);
          p += c * t; v += c * d;
          t0 = t1; t1 = t; d0 = d1; d1 = d;
        }
        position.push(p); velocity.push(v * 2 / granuleWidth);
      }
      return { position, velocity };
    }
    return {
      coverage: Object.freeze({ start, end, timeScale: 'TDB' }),
      /** Geometric state relative to the solar-system barycenter, except moonGeocentric. */
      state(body, jd) {
        if (closed) throw new Error('Ephemeris is closed');
        if (!Number.isFinite(jd) || jd < start || jd > end) throw new RangeError('Julian date outside ephemeris coverage');
        if (body === 'solarSystemBarycenter') return { position: [0, 0, 0], velocity: [0, 0, 0] };
        if (body === 'earth' || body === 'moon') {
          const earth = combine(series(2, jd), series(9, jd), -1 / (1 + earthMoonMassRatio));
          return body === 'earth' ? earth : combine(earth, series(9, jd), 1);
        }
        if (!Object.hasOwn(SERIES, body)) throw new RangeError(`Unknown DE441 body: ${body}`);
        return series(SERIES[body], jd);
      },
      close() { if (!closed) { closeSync(fd); closed = true; } },
    };
  } catch (error) { closeSync(fd); throw error; }
}

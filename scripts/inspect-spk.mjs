#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { inspectSpk, stateAtTdbSeconds } from '../src/index.mjs';

// Manual inspection utility. All epochs must be provided in TDB seconds from J2000.
// No automatic network downloads, no UTC guesses and no production writes.
const [filename, etArg, targetArg, centerArg] = process.argv.slice(2);
if (!filename) {
  console.error('Usage: node scripts/inspect-spk.mjs /path/to/de440s.bsp [etTdbSeconds targetNaifId centerNaifId]');
  process.exitCode = 2;
} else {
  try {
    const bytes = await readFile(filename);
    const hash = createHash('sha256').update(bytes).digest('hex');
    const metadata = inspectSpk(bytes);
    const output = {
      path: filename, bytes: bytes.length, sha256: hash,
      fileFormat: metadata.format, segmentCount: metadata.segments.length,
      segments: metadata.segments.map(s => ({
        name: s.name, target: s.target, center: s.center, frame: s.frame,
        type: s.type, startEt: s.startEt, endEt: s.endEt,
      })),
    };
    if (etArg !== undefined || targetArg !== undefined || centerArg !== undefined) {
      if ([etArg, targetArg, centerArg].some(x => x === undefined)) throw new Error('Provide all three state parameters together');
      const et = Number(etArg), target = Number(targetArg), center = Number(centerArg);
      output.state = stateAtTdbSeconds(bytes, target, center, et);
    }
    console.log(JSON.stringify(output, null, 2));
  } catch (error) {
    console.error(JSON.stringify({ code: error.code ?? 'INSPECTION_ERROR', message: error.message }));
    process.exitCode = 1;
  }
}

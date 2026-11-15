// Original VirgoStars source; CPAL-1.0 — see LICENSE and NOTICE.
// Copyright (c) 2026 Cesare-ml (VirgoStars project). All Rights Reserved.
// 2026-11-15: strict offline structural validation for CDS I/311 hip2.dat.
export const HIP2_ROW_LENGTH=276;
export const HIP2_EXPECTED_ROWS=117955;
export const HIP2_MAX_BYTES=60*1024*1024;

/**
 * Validate every fixed-width Hipparcos I/311/hip2 raw row before selection.
 * Checks text integrity, row length and global HIP uniqueness. This does not
 * claim that the underlying astrometric measurements are scientifically valid.
 */
export function readHip2Rows(bytes,{requireFullCatalog=true}={}) {
  if(!Buffer.isBuffer(bytes) || !bytes.length || bytes.length>HIP2_MAX_BYTES)
    throw new TypeError('Expected nonempty raw uncompressed hip2.dat Buffer (max 60 MiB)');
  if(typeof requireFullCatalog!=='boolean') throw new TypeError('requireFullCatalog must be boolean');
  let raw;
  try { raw=new TextDecoder('utf-8',{fatal:true}).decode(bytes); }
  catch { throw new Error('Invalid UTF-8 in Hipparcos fixed-width data'); }
  if(/[^\x20-\x7E\r\n]/.test(raw))
    throw new Error('Non-ASCII data in Hipparcos fixed-width source');
  const lines=raw.split(/\r?\n/);
  if(lines.at(-1)==='') lines.pop(); // Do not trim trailing fixed-width spaces.
  if(requireFullCatalog && lines.length!==HIP2_EXPECTED_ROWS)
    throw new Error(`Incomplete hip2.dat: expected ${HIP2_EXPECTED_ROWS} rows, found ${lines.length}`);
  const seen=new Set();
  for(let i=0;i<lines.length;i++){
    const row=lines[i];
    if(row.length!==HIP2_ROW_LENGTH) throw new Error(`hip2.dat row ${i+1}: expected ${HIP2_ROW_LENGTH} characters, got ${row.length}`);
    const id=row.slice(0,6).trim();
    if(!/^[1-9]\d*$/.test(id)) throw new Error(`hip2.dat row ${i+1}: invalid HIP ID`);
    const hip=Number(id);
    if(seen.has(hip)) throw new Error(`hip2.dat row ${i+1}: duplicate HIP ${hip}`);
    seen.add(hip);
  }
  return lines;
}

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

import {createHash} from 'node:crypto';
import {closeSync,fstatSync,openSync,readSync} from 'node:fs';
import {isAbsolute} from 'node:path';
import {validateExperimentalStarCatalog} from './experimental-swiss-subset.mjs';

const MAX_BYTES=64*1024*1024;
const label=value=>typeof value==='string' && value.length>0 && value===value.trim();

/** Load explicitly supplied bytes; the digest pins them, but does not establish their rights or scientific quality. */
export function loadExperimentalStarCatalogFile({filePath,sha256,source,license}={}){
  if(typeof filePath!=='string' || !isAbsolute(filePath))throw new TypeError('An absolute catalog file path is required');
  if(typeof sha256!=='string' || !/^[0-9a-f]{64}$/.test(sha256))throw new TypeError('A lowercase SHA-256 digest is required');
  if(!label(source) || !label(license))throw new TypeError('Explicit source and license labels are required');
  const fd=openSync(filePath,'r');
  let bytes;
  try{
    const stat=fstatSync(fd);
    if(!stat.isFile() || stat.size<1 || stat.size>MAX_BYTES)throw new RangeError('Catalog must be a nonempty regular file at most 64 MiB');
    const buffer=Buffer.allocUnsafe(stat.size+1);
    let count=0;
    while(count<buffer.length){const read=readSync(fd,buffer,count,buffer.length-count,null);if(read===0)break;count+=read;}
    if(count!==stat.size)throw new Error('Catalog size changed while reading');
    bytes=buffer.subarray(0,count);
  }finally{closeSync(fd);}
  if(createHash('sha256').update(bytes).digest('hex')!==sha256)throw new Error('Catalog SHA-256 mismatch');
  const parsed=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));
  if(!parsed || typeof parsed!=='object' || Array.isArray(parsed) || !label(parsed.id) || !Array.isArray(parsed.records) || parsed.records.length===0)throw new TypeError('Catalog JSON requires a nonempty ID and record array');
  validateExperimentalStarCatalog(parsed);
  for(const record of parsed.records){Object.freeze(record.aliases);Object.freeze(record);}
  Object.freeze(parsed.records);Object.freeze(parsed);
  return Object.freeze({catalog:parsed,provenance:Object.freeze({sha256,source,license})});
}

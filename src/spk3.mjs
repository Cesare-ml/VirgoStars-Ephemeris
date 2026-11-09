/** Independent SPK type 3 interpolation: six Chebyshev series X,Y,Z,VX,VY,VZ. */
import { SpkError, chebyshevValueAndDerivative } from './spk.mjs';

export function evaluateType3(buffer, segment, et, format) {
  if (!Buffer.isBuffer(buffer) || !segment || !Number.isFinite(et))
    throw new SpkError('INVALID_ARGUMENT', 'Expected SPK buffer, segment and TDB epoch');
  if (format !== 'LTL-IEEE' && format !== 'BIG-IEEE')
    throw new SpkError('UNSUPPORTED_FORMAT', 'Expected IEEE DAF byte order');
  if (segment.type !== 3)
    throw new SpkError('UNSUPPORTED_SPK_TYPE', `Expected type 3, got ${segment.type}`);
  if (et < segment.startEt || et > segment.endEt)
    throw new SpkError('OUT_OF_KERNEL_RANGE', 'Epoch outside segment');
  const word = address => {
    if (!Number.isSafeInteger(address) || address < segment.startAddress ||
        address > segment.endAddress || address < 1 || address*8 > buffer.length)
      throw new SpkError('INVALID_SPK', 'SPK word address out of bounds');
    return format === 'LTL-IEEE' ? buffer.readDoubleLE((address-1)*8) : buffer.readDoubleBE((address-1)*8);
  };
  const a = segment.startAddress, b = segment.endAddress;
  const init = word(b-3), intervalSeconds = word(b-2);
  const recordSize = word(b-1), n = word(b);
  if (!Number.isFinite(init) || !Number.isFinite(intervalSeconds) || intervalSeconds <= 0 ||
      !Number.isSafeInteger(recordSize) || recordSize < 8 || (recordSize-2) % 6 !== 0 ||
      !Number.isSafeInteger(n) || n < 1 || b-a+1 !== recordSize*n+4)
    throw new SpkError('INVALID_SPK','Invalid type-3 segment directory');
  const index = Math.min(n-1,Math.floor((et-init)/intervalSeconds));
  if (index < 0 || et > init + n*intervalSeconds + 1e-7)
    throw new SpkError('OUT_OF_KERNEL_RANGE','Epoch outside type-3 directory');
  const base = a + index*recordSize;
  const midpoint = word(base), radius = word(base+1);
  if (!Number.isFinite(midpoint)||!Number.isFinite(radius)||radius<=0)
    throw new SpkError('INVALID_SPK','Invalid type-3 record midpoint or radius');
  const x = (et-midpoint)/radius;
  if (x < -1-1e-10 || x > 1+1e-10)
    throw new SpkError('INVALID_SPK','Type-3 record interval mismatch');
  const count=(recordSize-2)/6;
  const components=[];
  for(let axis=0;axis<6;axis++){
    const c=Array.from({length:count},(_,i)=>word(base+2+axis*count+i));
    components.push(chebyshevValueAndDerivative(c,Math.max(-1,Math.min(1,x))).value);
  }
  return Object.freeze({
    positionKm:Object.freeze(components.slice(0,3)),
    velocityKmPerSec:Object.freeze(components.slice(3,6)),
  });
}

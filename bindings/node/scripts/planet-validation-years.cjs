'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit grids for reproducible diagnostics.
function validationYears(name){
  if(name===undefined||name==='0-5000-step-250')return Array.from({length:21},(_,i)=>i*250);
  if(name==='0-3000-step-50')return Array.from({length:61},(_,i)=>i*50);
  if(name==='0-3000-step-10')return Array.from({length:301},(_,i)=>i*10);
  throw new Error('unknown validation grid');
}
module.exports={validationYears};

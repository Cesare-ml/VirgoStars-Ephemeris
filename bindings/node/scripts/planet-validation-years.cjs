'use strict';
// SPDX-License-Identifier: CPAL-1.0. Explicit grids for reproducible diagnostics.
function validationYears(name){
  if(name===undefined||name==='0-5000-step-250')return Array.from({length:21},(_,i)=>i*250);
  if(name==='0-3000-step-50')return Array.from({length:61},(_,i)=>i*50);
  if(name==='0-3000-step-10')return Array.from({length:301},(_,i)=>i*10);
  // 13000 BCE is astronomical year -12999. Include both requested endpoints.
  if(name==='13000bce-3999-step-250')return [-12999,...Array.from({length:67},(_,i)=>-12750+i*250),3999];
  // Horizons currently permits only 9999 BCE and later, i.e. year -9998.
  if(name==='9999bce-3999-step-500')return [-9998,...Array.from({length:27},(_,i)=>-9500+i*500),3999];
  throw new Error('unknown validation grid');
}
module.exports={validationYears};

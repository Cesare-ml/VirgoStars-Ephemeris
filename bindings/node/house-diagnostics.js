'use strict';
// SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
// Per-call, opt-in diagnostics for synchronous legacy chart construction.

const SYSTEMS = new Set(['V', 'P', 'K', 'R', 'O']);
const ERROR_CODES = new Set(['E_POSITION_UNAVAILABLE', 'E_CALC', 'E_RANGE', 'E_UNSUPPORTED_FLAG', 'E_UNSUPPORTED_BODY', 'E_UNKNOWN']);

function failure(message) {
  const error = new Error(`E_DIAGNOSTIC_STATE: ${message}`);
  error.code = 'E_DIAGNOSTIC_STATE';
  return error;
}

/**
 * Return a facade and an isolated synchronous collection scope. The facade
 * retains the original result shapes; run() returns a separate JSON-safe
 * diagnostics envelope containing no epochs, locations, angles or messages.
 */
function createHouseDiagnosticSession(facade) {
  if (!facade || typeof facade.swe_houses !== 'function' || typeof facade.swe_houses_pos !== 'function') {
    throw failure('a house-capable facade is required');
  }
  let active = null;
  function record(operation, code, rawSystem, once = false) {
    if (!active) return;
    const system = code === 'POLAR_CIRCLE_LATITUDE' ? 'ALL' : (SYSTEMS.has(rawSystem) ? rawSystem : 'OTHER');
    const key = `${operation}|${code}|${system}`;
    const previous = active.get(key);
    if (previous) {
      if (!once) previous.count++;
    } else active.set(key, { operation, code, system, count: 1 });
  }
  function recordError(operation, system, result) {
    if (result && result.error) {
      const code = ERROR_CODES.has(result.code) ? result.code : 'E_OTHER';
      record(operation, code, system);
    }
  }
  const wrapped = Object.freeze({ ...facade,
    swe_houses(...args) {
      const result = facade.swe_houses(...args);
      recordError('swe_houses', args[3], result);
      if (result && result.status === 'FALLBACK_PORPHYRY') record('swe_houses', 'FALLBACK_PORPHYRY', args[3]);
      if (result && Array.isArray(result.warnings) && result.warnings.some((w) => w && w.code === 'POLAR_CIRCLE_LATITUDE')) {
        record('swe_houses', 'POLAR_CIRCLE_LATITUDE', null, true);
      }
      return result;
    },
    swe_houses_pos(...args) {
      const result = facade.swe_houses_pos(...args);
      recordError('swe_houses_pos', args[3], result);
      return result;
    },
  });
  function run(calculate) {
    if (active) throw failure('nested or concurrent diagnostic scopes are not supported');
    if (typeof calculate !== 'function' || calculate.constructor?.name === 'AsyncFunction') {
      throw failure('run requires a synchronous function');
    }
    active = new Map();
    try {
      const result = calculate();
      if (result && typeof result.then === 'function') throw failure('run cannot accept a promise');
      return { result, diagnostics: { schema: 'virgostars-house-diagnostics/1', events: [...active.values()] } };
    } finally {
      active = null;
    }
  }
  return Object.freeze({ facade: wrapped, run });
}

module.exports = { createHouseDiagnosticSession };

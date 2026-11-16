'use strict';
// MANUAL tool, needs the network: compares the Node binding (apparent geocentric
// positions) with JPL Horizons observer tables. Not part of `npm test` or CI.
//
//   node scripts/compare-horizons-apparent.cjs <de441.eph> [out.json]
//
// It sends only public queries (Sun, Moon, planets, geocenter, a list of epochs) to
// https://ssd.jpl.nasa.gov/api/horizons.api and prints aggregate residuals. Nothing
// private is sent and no Horizons rows are written to disk.
//
// Why two kinds of comparison: Horizons' apparent RA/Dec use an "EOP-corrected
// IAU76/80" equator and equinox of date, and it states that its equinox is offset
// -53 mas from the IAU 2006/2000A frame. Absolute RA/Dec therefore differ from an
// IAU 2006/2000B engine by a known, frame-only amount. Angular SEPARATIONS between
// bodies are invariant under any frame rotation, so they test light-time,
// aberration and deflection independently of the precession-nutation model.

const fs = require('node:fs');
const path = require('node:path');
const { Engine, constants, createHouseAngleProvider } = require(path.join(__dirname, '..', 'index.js'));

const [, , ephPath, outPath] = process.argv;
if (!ephPath) {
  console.error('usage: node scripts/compare-horizons-apparent.cjs <de441.eph> [out.json]');
  process.exit(2);
}
const engine = new Engine(ephPath);
const houseProvider = createHouseAngleProvider();
const SPEED = constants.SEFLG_SPEED;
const rad = Math.PI / 180;

// [name, Horizons command, Swiss-style body number]; Mars..Pluto are barycenters.
const bodies = [
  ['Sun', 10, 0], ['Moon', 301, 1], ['Mercury', 199, 2], ['Venus', 299, 3], ['MarsBarycenter', 4, 4],
  ['JupiterBarycenter', 5, 5], ['SaturnBarycenter', 6, 6], ['UranusBarycenter', 7, 7],
  ['NeptuneBarycenter', 8, 8], ['PlutoBarycenter', 9, 9],
];
if (process.env.CHIRON_SPK_PATH) {
  // Optional: also compare 2060 Chiron from a caller-supplied JPL small-body SPK.
  engine.loadChiron(process.env.CHIRON_SPK_PATH);
  bodies.push(['Chiron', "2060;", 15]);
}
const tdbList = [];
for (let jd = 2415100.5; jd < 2487500; jd += 2960.7) tdbList.push(jd);
// First Horizons EOP-data day (1962-01-20) and last (2026-11-13), as stated in its header.
const EOP_FIRST_JD = 2437684.5;
const EOP_LAST_JD = 2461353.5;
const HORIZONS_EQUINOX_OFFSET_ARCSEC = 0.053;

const tdbToTt = (tdb) => {
  const g = ((357.5291092 + 35999.0502909 * ((tdb - 2451545.0) / 36525)) * Math.PI) / 180;
  return tdb - (0.001657 * Math.sin(g)) / 86400;
};
const wrap = (d) => ((d + 540) % 360) - 180;
const unit = (lonDeg, latDeg) => [Math.cos(latDeg * rad) * Math.cos(lonDeg * rad), Math.cos(latDeg * rad) * Math.sin(lonDeg * rad), Math.sin(latDeg * rad)];
const separationArcsec = (a, b) => {
  const c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  return (Math.atan2(Math.hypot(...c), a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / rad) * 3600;
};

async function horizons(command, ttList) {
  const params = new URLSearchParams({
    format: 'json', COMMAND: `'${command}'`, OBJ_DATA: "'NO'", MAKE_EPHEM: "'YES'", EPHEM_TYPE: "'OBSERVER'",
    CENTER: "'500@399'", TLIST: `'${ttList.map((x) => x.toFixed(9)).join(' ')}'`, TIME_TYPE: "'TT'",
    QUANTITIES: "'2,20'", ANG_FORMAT: "'DEG'", CSV_FORMAT: "'YES'", EXTRA_PREC: "'YES'",
  });
  const json = await (await fetch(`https://ssd.jpl.nasa.gov/api/horizons.api?${params}`)).json();
  const text = json.result ?? '';
  const header = text.split('\n').find((l) => l.includes('R.A.__(a-app)'));
  const start = text.indexOf('$$SOE');
  if (!header || start < 0) throw new Error(`Horizons ${command}: ${text.slice(0, 300)}`);
  const cols = header.split(',').map((c) => c.trim());
  const index = (test) => cols.findIndex(test);
  const ra = index((c) => c === 'R.A.__(a-app)');
  const dec = index((c) => c === 'DEC___(a-app)');
  const delta = index((c) => c.startsWith('delta'));
  const deldot = index((c) => c.startsWith('deldot'));
  return text.slice(start + 5, text.indexOf('$$EOE')).trim().split('\n').map((l) => {
    const r = l.split(',').map((c) => c.trim());
    return { ra: Number(r[ra]), dec: Number(r[dec]), delta: Number(r[delta]), deldot: Number(r[deldot]) };
  });
}

(async () => {
  const ttList = tdbList.map(tdbToTt);
  const hz = {};
  const mine = {};
  const perBody = {};
  const insideEop = { raOffset: [], decMax: 0 };
  for (const [name, command, ipl] of bodies) {
    hz[name] = await horizons(command, ttList);
    if (hz[name].length !== tdbList.length) throw new Error(`${name}: ${hz[name].length} rows`);
    mine[name] = [];
    const s = (perBody[name] = { raArcsecMax: 0, decArcsecMax: 0, rangeRelMax: 0, rangeRateKmSMax: 0 });
    tdbList.forEach((tdb, i) => {
      const r = engine.calcTdb(tdb, ipl, SPEED);
      mine[name].push([r.xx[0], r.xx[1]]);
      const eps = houseProvider.calculateHouseAngles({ ut1JulianDay: ttList[i], ttJulianDay: ttList[i], latitudeDeg: 0, longitudeDegEast: 0 }).trueObliquityDeg * rad;
      const lon = r.xx[0] * rad;
      const lat = r.xx[1] * rad;
      const x = Math.cos(lat) * Math.cos(lon);
      const y = Math.cos(lat) * Math.sin(lon) * Math.cos(eps) - Math.sin(lat) * Math.sin(eps);
      const z = Math.cos(lat) * Math.sin(lon) * Math.sin(eps) + Math.sin(lat) * Math.cos(eps);
      const myRa = ((Math.atan2(y, x) / rad) + 360) % 360;
      const myDec = Math.atan2(z, Math.hypot(x, y)) / rad;
      const h = hz[name][i];
      const dRa = wrap(myRa - h.ra) * Math.cos(myDec * rad) * 3600;
      const dDec = (myDec - h.dec) * 3600;
      s.raArcsecMax = Math.max(s.raArcsecMax, Math.abs(dRa));
      s.decArcsecMax = Math.max(s.decArcsecMax, Math.abs(dDec));
      s.rangeRelMax = Math.max(s.rangeRelMax, Math.abs(r.xx[2] / h.delta - 1));
      s.rangeRateKmSMax = Math.max(s.rangeRateKmSMax, Math.abs((r.xx[5] * 149597870.7) / 86400 - h.deldot));
      if (tdb >= EOP_FIRST_JD && tdb <= EOP_LAST_JD) {
        insideEop.raOffset.push(dRa);
        insideEop.decMax = Math.max(insideEop.decMax, Math.abs(dDec));
      }
    });
  }

  const names = bodies.map((b) => b[0]);
  const sepStats = {};
  for (let i = 0; i < names.length; i++) {
    for (let j = i + 1; j < names.length; j++) {
      let max = 0;
      tdbList.forEach((_, k) => {
        const a = separationArcsec(unit(...mine[names[i]][k]), unit(...mine[names[j]][k]));
        const b = separationArcsec(unit(hz[names[i]][k].ra, hz[names[i]][k].dec), unit(hz[names[j]][k].ra, hz[names[j]][k].dec));
        max = Math.max(max, Math.abs(a - b));
      });
      sepStats[`${names[i]}-${names[j]}`] = max;
    }
  }
  const nonMoon = Object.entries(sepStats).filter(([k]) => !k.includes('Moon'));
  const moon = Object.entries(sepStats).filter(([k]) => k.includes('Moon'));
  const summary = {
    dates: tdbList.length,
    bodies: names,
    separation_pairs: Object.keys(sepStats).length,
    separation_max_arcsec_non_moon_pairs: Math.max(...nonMoon.map(([, v]) => v)),
    separation_max_arcsec_moon_pairs: Math.max(...moon.map(([, v]) => v)),
    absolute_radec_vs_horizons_max_arcsec: Object.fromEntries(Object.entries(perBody).map(([k, v]) => [k, { ra_cos_dec: v.raArcsecMax, dec: v.decArcsecMax }])),
    range_max_relative_difference: Object.fromEntries(Object.entries(perBody).map(([k, v]) => [k, v.rangeRelMax])),
    range_rate_max_difference_km_s: Object.fromEntries(Object.entries(perBody).map(([k, v]) => [k, v.rangeRateKmSMax])),
    inside_horizons_eop_span: {
      epochs: insideEop.raOffset.length / names.length,
      ra_offset_arcsec_min: Math.min(...insideEop.raOffset),
      ra_offset_arcsec_max: Math.max(...insideEop.raOffset),
      ra_offset_after_removing_stated_53_mas_arcsec_range: [Math.min(...insideEop.raOffset) - HORIZONS_EQUINOX_OFFSET_ARCSEC, Math.max(...insideEop.raOffset) - HORIZONS_EQUINOX_OFFSET_ARCSEC],
      dec_abs_max_arcsec: insideEop.decMax,
    },
  };
  console.log(JSON.stringify(summary, null, 1));
  if (outPath) fs.writeFileSync(outPath, `${JSON.stringify(summary, null, 1)}\n`);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

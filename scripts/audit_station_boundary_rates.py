# SPDX-License-Identifier: CPAL-1.0
"""Diagnostic Horizons/ERFA rate comparison near Rust stations and TT bounds.

Usage: python scripts/audit_station_boundary_rates.py KERNEL SHA256 RUST_EXE NEW_JSON
All astronomy comes from the existing Rust core, Horizons or ERFA. The sample
station locations are selected from Rust; they are not independent root estimates.
"""
import csv
import datetime
import io
import json
import math
import pathlib
import re
import subprocess
import sys
import urllib.parse
import urllib.request

from audit_horizons_ecliptic_radial_rates import TARGETS, AU_KM, ecliptic, four_point, sha

MIN_TT, MAX_TT = 2415020.0, 2488070.0
DEADBAND = 0.0005  # arcseconds/TDB day; diagnostic, not an error bound


def sign(value):
    return 1 if value > DEADBAND else -1 if value < -DEADBAND else 0


def rust(executable, kernel, cases):
    request = ''.join(f'{body},{jd:.9f}\n' for body, jd in cases)
    result = subprocess.run([str(executable), str(kernel)], input=request, text=True, capture_output=True, check=True)
    lines = result.stdout.splitlines()
    if len(lines) != len(cases):
        raise SystemExit('Rust output count mismatch')
    output = []
    for line, (body, requested) in zip(lines, cases):
        fields = line.split(',', 4)
        if len(fields) < 4 or fields[0] != str(body) or abs(float(fields[1]) - requested) > 1e-8:
            raise SystemExit('Rust output epoch or ID mismatch')
        row = {'id': str(body), 'jdTdb': float(fields[1]), 'jdTt': float(fields[2]), 'status': fields[3]}
        if fields[3] == 'ok':
            numbers = [float(x) for x in fields[4].split(',')]
            if len(numbers) != 6 or not all(math.isfinite(x) for x in numbers):
                raise SystemExit('invalid Rust six fields')
            row['fields'] = numbers
        elif fields[3] == 'error':
            row['error'] = fields[4]
        elif fields[3] != 'time' or str(body) != 'tt':
            raise SystemExit('invalid Rust status')
        output.append(row)
    return output


def station_brackets(exe, kernel):
    start, stop = 2460310.5, 2461405.5
    times = [start + 5 * i for i in range(int((stop - start) / 5) + 1)]
    sampled = rust(exe, kernel, [(body, t) for body in range(2, 10) for t in times])
    events = []
    for body in range(2, 10):
        series = [row for row in sampled if row['id'] == str(body)]
        if len(series) != len(times) or any(row['status'] != 'ok' for row in series):
            raise SystemExit('station scan failed')
        found = {}
        for left, right in zip(series, series[1:]):
            a, b = left['fields'][3], right['fields'][3]
            if a > 0 > b and 'direct_to_retrograde' not in found:
                found['direct_to_retrograde'] = [left['jdTdb'], right['jdTdb'], a, b]
            if a < 0 < b and 'retrograde_to_direct' not in found:
                found['retrograde_to_direct'] = [left['jdTdb'], right['jdTdb'], a, b]
        if len(found) != 2:
            raise SystemExit(f'missing station bracket for ID {body}: {sorted(found)}')
        events.extend({'id': body, 'kind': k, 'lo': v[0], 'hi': v[1], 'loRate': v[2], 'hiRate': v[3]}
                      for k, v in found.items())
    while any(e['hi'] - e['lo'] > 0.0001 for e in events):
        pending = [e for e in events if e['hi'] - e['lo'] > 0.0001]
        updates = rust(exe, kernel, [(e['id'], (e['lo'] + e['hi']) / 2) for e in pending])
        for event, row in zip(pending, updates):
            if row['status'] != 'ok':
                raise SystemExit('bisection evaluation failed')
            value = row['fields'][3]
            if value == 0.0:
                event['lo'] = row['jdTdb']; event['hi'] = row['jdTdb']
                event['loRate'] = 0.0; event['hiRate'] = 0.0
            elif value * event['loRate'] > 0:
                event['lo'] = row['jdTdb']; event['loRate'] = value
            else:
                event['hi'] = row['jdTdb']; event['hiRate'] = value
    for event in events:
        event['center'] = (event['lo'] + event['hi']) / 2
    return events


def main():
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    kernel, expected, exe, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    if not re.fullmatch('[a-f0-9]{64}', expected) or out.exists() or len({kernel.resolve(), exe.resolve(), out.resolve()}) != 3:
        raise SystemExit('invalid arguments')
    if sha(kernel) != expected:
        raise SystemExit('kernel mismatch')
    exe_hash = sha(exe)
    events = station_brackets(exe, kernel)
    cases = []
    for event_no, event in enumerate(events):
        for hours in (-6, -1, 0, 1, 6):
            cases.append({'kind': 'station', 'id': event['id'], 'event': event_no,
                          'hours': hours, 'jdTdb': event['center'] + hours / 24.0,
                          'offsets': [-0.08, -0.04, -0.02, 0.0, 0.02, 0.04, 0.08]})
    for body in range(10):
        for bound, direction in ((MIN_TT, 1), (MAX_TT, -1)):
            for margin in (0.0101, 0.03, 0.1, 1.0):
                cases.append({'kind': 'boundary', 'id': body, 'bound': bound, 'marginTdbDay': margin,
                              'jdTdb': bound + direction * margin,
                              'offsets': [-0.01, -0.005, -0.0025, 0.0, 0.0025, 0.005, 0.01]})
    center_rows = rust(exe, kernel, [(c['id'], c['jdTdb']) for c in cases])
    for case, row in zip(cases, center_rows):
        if row['status'] != 'ok':
            raise SystemExit(f'planned center failed: {case} {row}')
        case['rust'] = row
        if case['kind'] == 'boundary' and not (MIN_TT <= row['jdTt'] - 0.01 and row['jdTt'] + 0.01 < MAX_TT):
            raise SystemExit('boundary center lacks internal orientation stencil')
    probes = []
    for body in range(10):
        for bound, direction in ((MIN_TT, 1), (MAX_TT, -1)):
            for label, delta in (('exact', 0.0), ('inside_0.005', direction * 0.005), ('outside_0.0001', -direction * 0.0001)):
                probes.append({'id': body, 'bound': bound, 'label': label, 'jdTdb': bound + delta})
    probe_rows = rust(exe, kernel, [(p['id'], p['jdTdb']) for p in probes])
    for probe, row in zip(probes, probe_rows):
        probe['result'] = row
    epoch_keys = sorted({(c['id'], c['jdTdb'] + offset) for c in cases for offset in c['offsets']})
    times = rust(exe, kernel, [('tt', jd) for _, jd in epoch_keys])
    tt_map = {key: row['jdTt'] for key, row in zip(epoch_keys, times)}
    for case in cases:
        if case['kind'] == 'boundary' and any(not (MIN_TT <= tt_map[(case['id'], case['jdTdb'] + offset)] < MAX_TT) for offset in case['offsets']):
            raise SystemExit('reference boundary sample outside orientation interval')
    ref, sources = {}, []
    for body, (_, command, name) in enumerate(TARGETS):
        keys = [key for key in epoch_keys if key[0] == body]
        for start in range(0, len(keys), 60):
            group = keys[start:start + 60]
            epochs = [tt_map[key] for key in group]
            params = {'format': 'json', 'COMMAND': f"'{command}'", 'OBJ_DATA': "'NO'", 'MAKE_EPHEM': "'YES'",
                      'EPHEM_TYPE': "'OBSERVER'", 'CENTER': "'500@399'", 'TLIST': "'" + ' '.join(f'{tt:.9f}' for tt in epochs) + "'",
                      'TLIST_TYPE': "'JD'", 'TIME_TYPE': "'TT'", 'CAL_FORMAT': "'JD'", 'QUANTITIES': "'20,23,45'",
                      'REF_SYSTEM': "'ICRF'", 'ANG_FORMAT': "'DEG'", 'CSV_FORMAT': "'YES'", 'EXTRA_PREC': "'YES'", 'APPARENT': "'AIRLESS'"}
            url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
            with urllib.request.urlopen(url, timeout=30) as response:
                raw = json.load(response)['result']
            if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}', raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}', raw):
                raise SystemExit('unexpected Horizons target, center or source')
            header = next((line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line), '')
            columns = [x.strip() for x in header.split(',')]
            if 'Date_________JDTT' not in columns:
                raise SystemExit('wrong reference time scale')
            index = [columns.index(x) for x in ('delta', 'deldot', 'S-O-T', 'RA_(ICRF-a-app)', 'DEC_(ICRF-a-app)')]
            a, b = raw.find('$$SOE'), raw.find('$$EOE')
            if a < 0 or b <= a:
                raise SystemExit('missing Horizons data')
            lines = list(csv.reader(io.StringIO(raw[a + 5:b].strip())))
            if len(lines) != len(epochs):
                raise SystemExit('Horizons row count mismatch')
            for key, tt, fields in zip(group, epochs, lines):
                if not fields[0].strip() or any(not fields[i].strip() for i in index) or abs(float(fields[0]) - tt) > 1e-8:
                    raise SystemExit('missing Horizons value or wrong epoch')
                v = [float(fields[i]) for i in index]
                if not all(math.isfinite(z) for z in v):
                    raise SystemExit('nonfinite Horizons data')
                lon, lat = ecliptic(v[3], v[4], tt)
                ref[key] = {'lon': lon, 'lat': lat, 'distance': v[0], 'q20Rate': v[1], 'elongation': v[2]}
            sources.append({'target': name, 'url': url, 'raw': raw})
    for case in cases:
        center, body = case['jdTdb'], case['id']
        group = [ref[(body, center + offset)] for offset in case['offsets']]
        # Horizons received these nine-decimal TT epochs, not the unrounded
        # TDB inputs. Use the actual sample spacing; treating rounded TT as
        # exact unrounded TDB can dominate short-step derivatives.
        jds = [tt_map[(body, center + offset)] for offset in case['offsets']]
        lon0 = group[3]['lon']
        values = [[lon0 + ((row['lon'] - lon0 + 180.0) % 360.0 - 180.0) for row in group],
                  [row['lat'] for row in group], [row['distance'] for row in group]]
        fields = case['rust']['fields']; got = fields[3:6]
        rates = []
        for points in ([1, 2, 4, 5], [0, 1, 5, 6]):
            d = [four_point(series, jds, points, jds[3]) for series in values]
            rates.append({'h': abs(case['offsets'][points[1]]), 'reference': d,
                          'longitudeErrorArcsecDay': (got[0] - d[0]) * 3600,
                          'latitudeErrorArcsecDay': (got[1] - d[1]) * 3600,
                          'radialErrorAuDay': got[2] - d[2],
                          'q20MinusDerivativeKmS': group[3]['q20Rate'] - d[2] * AU_KM / 86400.0,
                          'rustSign': sign(got[0] * 3600), 'referenceSign': sign(d[0] * 3600)})
        case['rates'] = rates
        case['elongationDeg'] = group[3]['elongation']
    summary = {}
    for case_kind in ('station', 'boundary'):
        selected = [c for c in cases if c['kind'] == case_kind]
        entries = [(c, s) for c in selected for s in c['rates']]
        def metric(key, limit):
            worst = max(entries, key=lambda pair: abs(pair[1][key]))
            return {'limit': limit, 'checks': len(entries), 'failures': sum(abs(s[key]) > limit for _, s in entries),
                    'maxAbs': abs(worst[1][key]), 'id': worst[0]['id'], 'jdTdb': worst[0]['jdTdb'], 'h': worst[1]['h']}
        summary[case_kind] = {'cases': len(selected), 'longitudeArcsecDay': metric('longitudeErrorArcsecDay', 0.05),
                              'latitudeArcsecDay': metric('latitudeErrorArcsecDay', 0.05),
                              'radialAuDay': metric('radialErrorAuDay', 5e-10),
                              'q20DirectKmS': metric('q20MinusDerivativeKmS', 0.001),
                              'signMismatches': sum(s['rustSign'] * s['referenceSign'] == -1 for _, s in entries),
                              'signIndeterminate': sum(not s['rustSign'] or not s['referenceSign'] for _, s in entries)}
    windows = {}
    for hours in (1, 6):
        for stencil in (0, 1):
            counts = {'confirmed': 0, 'indeterminate': 0, 'contradicted': 0}
            for i, event in enumerate(events):
                pair = [next(c for c in cases if c['kind'] == 'station' and c['event'] == i and c['hours'] == side * hours) for side in (-1, 1)]
                expected_signs = (1, -1) if event['kind'] == 'direct_to_retrograde' else (-1, 1)
                signs = [(c['rates'][stencil]['rustSign'], c['rates'][stencil]['referenceSign']) for c in pair]
                if any(v and v != expected for values, expected in zip(signs, expected_signs) for v in values):
                    counts['contradicted'] += 1
                elif all(v for values in signs for v in values):
                    counts['confirmed'] += 1
                else:
                    counts['indeterminate'] += 1
            windows[f'{hours}h_stencil{stencil}'] = counts
    failed_probes = [p for p in probes if p['result']['status'] != 'error' or 'Orientation(UnsupportedEpoch' not in p['result'].get('error', '')]
    summary['windows'] = windows
    summary['boundaryProbes'] = {'checks': len(probes), 'unexpected': len(failed_probes)}
    if sha(kernel) != expected or sha(exe) != exe_hash:
        raise SystemExit('input changed')
    result = {'schema': 'de441-station-boundary-rates/1', 'status': 'EXPLORATORY',
              'referenceDerivativeScale': 'TT day (actual requested TT epochs)', 'rustDerivativeScale': 'TDB day',
              'capturedUtc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'kernelSha256': expected,
              'binarySha256': exe_hash, 'events': events, 'summary': summary, 'cases': cases, 'probes': probes, 'sources': sources}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if failed_probes or any(summary[k][m]['failures'] for k in ('station', 'boundary') for m in ('longitudeArcsecDay', 'latitudeArcsecDay', 'radialAuDay', 'q20DirectKmS')) or any(v['contradicted'] for v in windows.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()

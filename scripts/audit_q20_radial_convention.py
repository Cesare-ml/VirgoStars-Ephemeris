# SPDX-License-Identifier: CPAL-1.0
"""Project Rust diagnostic light-time velocities for a saved Horizons report.

Usage: python scripts/audit_q20_radial_convention.py KERNEL SHA256 EXE REF REF_SHA256 NEW_JSON
The deliberately incomplete projection is not a physical apparent derivative.
"""
import csv
import hashlib
import io
import json
import math
import pathlib
import re
import subprocess
import sys


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(4 * 1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def main():
    if len(sys.argv) != 7:
        raise SystemExit(__doc__)
    kernel, kernel_hash, executable, reference, reference_hash, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4]), sys.argv[5], pathlib.Path(sys.argv[6])
    if not all(re.fullmatch('[a-f0-9]{64}', h) for h in (kernel_hash, reference_hash)) or out.exists() or len({p.resolve() for p in (kernel, executable, reference, out)}) != 4:
        raise SystemExit('invalid arguments')
    if sha(kernel) != kernel_hash or sha(reference) != reference_hash:
        raise SystemExit('input hash mismatch')
    exe_hash = sha(executable)
    report = json.loads(reference.read_text(encoding='utf8'))
    if report.get('schema') != 'horizons-erfa-ecliptic-radial-rates/1' or report.get('kernelSha256') != kernel_hash:
        raise SystemExit('wrong reference provenance')
    bodies = {'Sun': 'sun', 'Moon': 'moon', 'Mercury': 'mercury', 'Venus': 'venus', 'Jupiter Barycenter': 'jupiter'}
    chosen = [r for r in report['records'] if r['target'] in bodies]
    if len(chosen) != 45 or len({(r['target'], r['year']) for r in chosen}) != 45:
        raise SystemExit('wrong reference sample')
    rows = []
    for case in chosen:
        proc = subprocess.run([str(executable), str(kernel), 'earth', bodies[case['target']], str(case['jdTdb'])], capture_output=True, text=True, check=True)
        table = list(csv.DictReader(io.StringIO(proc.stdout)))
        if len(table) != 1:
            raise SystemExit('invalid diagnostic output')
        entry = table[0]
        p = [float(entry[k]) for k in ('x', 'y', 'z')]
        full = [float(entry[k]) for k in ('correct_vx', 'correct_vy', 'correct_vz')]
        hybrid = [float(entry[k]) for k in ('hybrid_vx', 'hybrid_vy', 'hybrid_vz')]
        length = math.hypot(*p)
        if not all(math.isfinite(v) for v in p + full + hybrid) or not length > 0:
            raise SystemExit('invalid position or velocity')
        project = lambda velocity: sum(a * b for a, b in zip(p, velocity)) / length / 86400.0
        full_speed, incomplete_speed = project(full), project(hybrid)
        reference_speed = case['stencils'][0]['reference'][2] * 149597870.7 / 86400.0
        rows.append({'target': case['target'], 'year': case['year'], 'jdTdb': case['jdTdb'],
                     'correctKmS': full_speed, 'incompleteKmS': incomplete_speed,
                     'q20KmS': case['q20RangeRateKmS'], 'q20RangeDerivativeKmS': reference_speed,
                     'correctMinusDerivativeKmS': full_speed - reference_speed,
                     'incompleteMinusQ20KmS': incomplete_speed - case['q20RangeRateKmS'],
                     'lightTimeRate': float(entry['light_time_rate'])})
    if sha(kernel) != kernel_hash or sha(reference) != reference_hash or sha(executable) != exe_hash:
        raise SystemExit('input changed')
    limit = 0.00001
    def summarize(key):
        largest = max(rows, key=lambda row: abs(row[key]))
        return {'limitKmS': limit, 'cases': len(rows), 'failures': sum(abs(row[key]) > limit for row in rows),
                'maxAbsKmS': abs(largest[key]), 'target': largest['target'], 'year': largest['year']}
    summary = {'correctVsRangeDerivative': summarize('correctMinusDerivativeKmS'), 'incompleteVsQ20': summarize('incompleteMinusQ20KmS')}
    result = {'schema': 'q20-lighttime-projection/1', 'status': 'EXPLORATORY', 'kernelSha256': kernel_hash,
              'referenceSha256': reference_hash, 'binarySha256': exe_hash, 'summary': summary, 'rows': rows}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if any(item['failures'] for item in summary.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()

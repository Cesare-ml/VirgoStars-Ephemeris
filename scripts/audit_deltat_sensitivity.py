# SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""Illustrate fixed-UT1 Moon sensitivity. No uncertainty bounds inferred.

python scripts/audit_deltat_sensitivity.py REPORT MANIFEST PACK_DIR EXE NEW_REPORT
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

EXPECTED_MANIFEST = 'a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a'


def sha(p):
    h = hashlib.sha256()
    with p.open('rb') as f:
        for chunk in iter(lambda: f.read(1024*1024), b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    report, manifest, packs, exe, out = map(lambda x: Path(x).resolve(), sys.argv[1:])
    if out.exists() or out in (report, manifest, exe) or packs in out.parents:
        raise SystemExit('output must be new and outside inputs')
    meta = json.loads(manifest.read_text())
    if sha(manifest) != EXPECTED_MANIFEST or len(meta['packages']) != 10:
        raise ValueError('unexpected manifest')
    inputs = [(report, sha(report)), (exe, sha(exe)), (manifest, EXPECTED_MANIFEST)]
    for entry in meta['packages']:
        art = entry['artifact']
        if Path(art['file']).name != art['file']:
            raise ValueError('invalid filename')
        p = packs / art['file']
        if p.stat().st_size != art['bytes']:
            raise ValueError('size mismatch')
        inputs.append((p, art['sha256']))
    def verify():
        for p, digest in inputs:
            if sha(p) != digest:
                raise ValueError(f'hash mismatch: {p}')
    verify()
    source = json.loads(report.read_text())
    rows = []
    for year, perturbation in [(0, 260.0), (500, 140.0), (1000, 55.0), (1500, 20.0)]:
        entry = next(x for x in source['horizons'] if int(x['decimalYear']) == year)
        rows.append({'label': str(year), 'ut1': entry['jdTt'], 'deltaT': entry['model'], 'perturbation': perturbation})
    event = next(x for x in source['events'] if x['year'] == 1514)
    for label, delta in [('1514-model', event['model']), ('1514-lower', 277.0), ('1514-upper', 890.0)]:
        rows.append({'label': label, 'ut1': 2274277.68, 'deltaT': delta, 'perturbation': 0.0})
    input_text = ''.join(f"{x['ut1']:.12f},{x['deltaT']:.12f},{x['perturbation']:.12f}\n" for x in rows)
    proc = subprocess.run([str(exe), str(packs)], input=input_text, text=True, capture_output=True, check=True)
    outputs = proc.stdout.splitlines()
    if len(outputs) != len(rows):
        raise ValueError('wrong output count')
    for row, line in zip(rows, outputs):
        fields = list(map(float, line.split(',')))
        if len(fields) != 12 or abs(fields[0] - row['ut1']) > 1e-8:
            raise ValueError('wrong Rust output')
        row['lower'], row['nominal'], row['upper'] = fields[3:6], fields[6:9], fields[9:12]
        row['lowerMinusNominalLongitudeArcmin'] = ((row['lower'][0]-row['nominal'][0]+180)%360-180)*60
        row['upperMinusNominalLongitudeArcmin'] = ((row['upper'][0]-row['nominal'][0]+180)%360-180)*60
    baseline = rows[4]['nominal'][0]
    for row in rows[5:]:
        row['minus1514ModelLongitudeArcmin'] = ((row['nominal'][0]-baseline+180)%360-180)*60
    verify()
    result = {'schema': 'de441-deltat-moon-sensitivity/1', 'status': 'EXPERIMENTAL',
              'manifestSha256': EXPECTED_MANIFEST, 'inputs': {str(p): h for p,h in inputs}, 'rows': rows}
    with out.open('x', encoding='utf8', newline='\n') as f:
        json.dump(result, f, indent=2, allow_nan=False)
        f.write('\n')
    print(json.dumps(rows))


if __name__ == '__main__':
    main()

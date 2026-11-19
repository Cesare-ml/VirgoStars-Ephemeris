# SPDX-License-Identifier: CPAL-1.0
"""Compare existing experimental Rust outputs at logical pack edges.

Usage: python scripts/audit_orientation_pack_edges.py KERNEL MANIFEST PACK_DIR EXE NEW_JSON
This opens slice files directly; it does not exercise the mobile package manager.
"""
import json
import pathlib
import sys

from audit_horizons_ecliptic_radial_rates import sha
from audit_package_orientation_horizons import EXPECTED_SOURCE, engine, wrap


def main():
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    kernel, manifest, packs, exe, out = map(pathlib.Path, sys.argv[1:])
    if out.exists() or len({kernel.resolve(), manifest.resolve(), exe.resolve(), out.resolve()}) != 4:
        raise SystemExit('output exists or aliases input')
    manifest_hash, exe_hash = sha(manifest), sha(exe)
    meta = json.loads(manifest.read_text(encoding='utf8'))
    if meta.get('schema') != 'virgostars.de441.mobile-packs.v1' or meta['source']['sha256'] != EXPECTED_SOURCE or len(meta['packages']) != 10:
        raise SystemExit('wrong manifest')
    if kernel.stat().st_size != meta['source']['bytes'] or sha(kernel) != EXPECTED_SOURCE:
        raise SystemExit('wrong source')
    all_cases, pack_paths = [], {}
    for package in meta['packages']:
        art, package_id = package['artifact'], package['id']
        if pathlib.Path(art['file']).name != art['file'] or package_id in pack_paths:
            raise SystemExit('invalid package')
        file = packs / art['file']
        if file.stat().st_size != art['bytes'] or sha(file) != art['sha256']:
            raise SystemExit('wrong pack hash or size')
        pack_paths[package_id] = (file, art['sha256'])
        for label, jd in (('start', art['requested_start_jd_tdb']), ('start_plus_1s', art['requested_start_jd_tdb'] + 1 / 86400),
                          ('end_minus_1s', art['requested_end_exclusive_jd_tdb'] - 1 / 86400)):
            all_cases.append({'package': package_id, 'label': label, 'jdTdb': jd})
    all_input = [(body, sample['jdTdb']) for sample in all_cases for body in range(10)]
    full = engine(exe, kernel, all_input)
    checks = []
    for package_id, (file, _) in pack_paths.items():
        selected = [(i, sample) for i, sample in enumerate(all_cases) if sample['package'] == package_id]
        local = engine(exe, file, [(body, sample['jdTdb']) for _, sample in selected for body in range(10)])
        for group, (index, sample) in enumerate(selected):
            for body in range(10):
                a, b = full[index * 10 + body]['candidate'], local[group * 10 + body]['candidate']
                errors = [abs(wrap(a[0] - b[0]))] + [abs(x - y) for x, y in zip(a[1:], b[1:])]
                limits = [1e-8, 1e-8, 1e-12, 1e-8, 1e-8, 1e-12]
                checks.append({'package': package_id, 'edge': sample['label'], 'jdTdb': sample['jdTdb'], 'id': body,
                               'errors': errors, 'overLimit': any(x > y for x, y in zip(errors, limits))})
    if sha(kernel) != EXPECTED_SOURCE or sha(manifest) != manifest_hash or sha(exe) != exe_hash or any(sha(file) != h for file, h in pack_paths.values()):
        raise SystemExit('input changed')
    summary = {'checks': len(checks), 'failures': sum(x['overLimit'] for x in checks),
               'maxAbsByField': [max(x['errors'][i] for x in checks) for i in range(6)]}
    result = {'schema': 'package-orientation-edges/1', 'status': 'EXPLORATORY', 'kernelSha256': EXPECTED_SOURCE,
              'manifestSha256': manifest_hash, 'binarySha256': exe_hash, 'summary': summary, 'checks': checks}
    with out.open('x', encoding='utf8', newline='\n') as destination:
        json.dump(result, destination, indent=2, allow_nan=False)
        destination.write('\n')
    print(json.dumps(summary))
    if summary['failures']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

# SPDX-License-Identifier: CPAL-1.0
"""Run the cross-branch Rust-only audit in a new local directory.

python scripts/run_multipack_boundary_audit.py MANAGER_REPO KERNEL MANIFEST PACK_DIR NEW_WORK_DIR
The manager is read from the pinned #608 audit commit, never the working tree.
No kernel is copied. The new work directory contains only a tiny synthetic corrupt
file, a temporary build and a CSV/evidence receipt; it must not already exist.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

MANAGER_COMMIT = '064b9c9584732e6bad39fce55c471759c8fab473'
MANAGER_BLOB_SHA = '1191e46208c244bcfdc9d0940b2f429e662d78d4092bba3e65131b450a0bad6b'
SOURCE_SHA = '476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da'


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as file:
        for block in iter(lambda: file.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def main():
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    manager_repo, kernel, manifest, pack_dir, work = map(lambda p: Path(p).resolve(), sys.argv[1:])
    repo = Path(__file__).resolve().parent.parent
    if work.exists() or work == kernel or work == manifest or work == pack_dir or work in kernel.parents or work in manifest.parents or work in pack_dir.parents or pack_dir in work.parents:
        raise SystemExit('work must be new and not contain any input')
    meta = json.loads(manifest.read_text(encoding='utf8'))
    if meta.get('schema') != 'virgostars.de441.mobile-packs.v1' or len(meta['packages']) != 10 or meta['source']['sha256'] != SOURCE_SHA:
        raise SystemExit('unexpected manifest')
    sources = [(kernel, SOURCE_SHA, meta['source']['bytes']), (manifest, sha(manifest), manifest.stat().st_size)]
    for entry in meta['packages']:
        artifact = entry['artifact']
        if Path(artifact['file']).name != artifact['file']:
            raise SystemExit('invalid package file name')
        sources.append((pack_dir / artifact['file'], artifact['sha256'], artifact['bytes']))
    def verify():
        for path, expected, size in sources:
            if path.stat().st_size != size or sha(path) != expected:
                raise RuntimeError(f'input changed or mismatched: {path}')
    verify()
    manager = subprocess.check_output(['git', '-C', str(manager_repo), 'show', MANAGER_COMMIT + ':crates/ephemeris-core/src/de441_pack_manager.rs'])
    if hashlib.sha256(manager).hexdigest() != MANAGER_BLOB_SHA:
        raise SystemExit('unexpected manager source')
    work.mkdir(parents=True, exist_ok=False)
    (work / 'src').mkdir()
    (work / 'src' / 'pack_manager.rs').write_bytes(manager)
    (work / 'src' / 'main.rs').write_bytes((repo / 'audits' / 'multipack_boundary.rs').read_bytes())
    (work / 'Cargo.toml').write_text('[package]\nname = "provider-boundary-audit"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n\n[dependencies]\nvirgostars-ephemeris-core = { path = ' + json.dumps((repo / 'crates/ephemeris-core').as_posix()) + ' }\n', encoding='utf8')
    (work / 'corrupt-test-only.eph').write_bytes(b'not a DE441 file')
    command = ['cargo', 'run', '--release', '--offline', '--manifest-path', str(work / 'Cargo.toml'), '--', str(pack_dir), str(kernel), str(work / 'deliberately-absent.eph'), str(work / 'corrupt-test-only.eph')]
    env = dict(os.environ)
    with (work / 'results.csv').open('x', encoding='utf8', newline='\n') as out, (work / 'stderr.log').open('x', encoding='utf8', newline='\n') as err:
        result = subprocess.run(command, stdout=out, stderr=err, env=env, check=False)
    verify()
    receipt = {'managerCommit': MANAGER_COMMIT, 'managerBlobSha256': hashlib.sha256(manager).hexdigest(),
               'orientationCommit': subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip(),
               'auditSourceSha256': sha(repo / 'audits' / 'multipack_boundary.rs'),
               'inputs': [{'path': str(p), 'sha256': h, 'bytes': n} for p, h, n in sources],
               'command': command, 'exitCode': result.returncode, 'resultsSha256': sha(work / 'results.csv'),
               'stderrSha256': sha(work / 'stderr.log')}
    with (work / 'receipt.json').open('x', encoding='utf8') as out:
        json.dump(receipt, out, indent=2)
        out.write('\n')
    print(json.dumps(receipt, indent=2))
    if result.returncode:
        raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()

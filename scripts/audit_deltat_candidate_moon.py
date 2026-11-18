# SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""Post-hoc 1514 Moon sensitivity from supplied audits and local DE441 packs.

python scripts/audit_deltat_candidate_moon.py UPDATED OLD MANIFEST PACKS EXE NEW_REPORT
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def sha(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    if len(sys.argv)!=7:
        raise SystemExit(__doc__)
    updated,old,manifest,packs,exe,out=map(lambda s:Path(s).resolve(),sys.argv[1:])
    if out.exists() or out in (updated,old,manifest,exe) or packs in out.parents:
        raise SystemExit('report must be new and outside inputs')
    meta=json.loads(manifest.read_text())
    manifest_hash='a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a'
    if sha(manifest)!=manifest_hash or len(meta['packages'])!=10:
        raise ValueError('unexpected manifest')
    inputs=[(updated,sha(updated)),(old,sha(old)),(exe,sha(exe)),(manifest,manifest_hash)]
    for entry in meta['packages']:
        art=entry['artifact']; p=packs/art['file']
        if Path(art['file']).name!=art['file'] or p.stat().st_size!=art['bytes']:
            raise ValueError('invalid package')
        inputs.append((p,art['sha256']))
    def verify():
        for p,h in inputs:
            if sha(p)!=h: raise ValueError(f'hash mismatch: {p}')
    verify()
    candidate=next(x['model'] for x in json.loads(updated.read_text())['events'] if x['year']==1514)
    older=next(x['model'] for x in json.loads(old.read_text())['events'] if x['year']==1514)
    cases=[('old',older),('s5_2021',candidate),('conditional_min',277.0)]
    text=''.join(f'2274277.68,{value:.12f},0.0\n' for _,value in cases)
    proc=subprocess.run([str(exe),str(packs)],input=text,text=True,capture_output=True,check=True)
    lines=proc.stdout.splitlines()
    if len(lines)!=len(cases): raise ValueError('wrong result count')
    rows=[]
    for (name,delta),line in zip(cases,lines):
        fields=list(map(float,line.split(',')))
        if len(fields)!=12 or abs(fields[0]-2274277.68)>1e-8 or abs(fields[1]-delta)>1e-8:
            raise ValueError('unexpected Rust result')
        rows.append({'name':name,'deltaT':delta,'moonLonDeg':fields[6],'moonLatDeg':fields[7],'moonDistanceAu':fields[8]})
    for row in rows:
        row['minusOldArcmin']=((row['moonLonDeg']-rows[0]['moonLonDeg']+180)%360-180)*60
        row['minusCandidateArcmin']=((row['moonLonDeg']-rows[1]['moonLonDeg']+180)%360-180)*60
    verify()
    with out.open('x',encoding='utf8',newline='\n') as f:
        json.dump({'schema':'de441-updated-deltat-moon/1','ut1Jd':2274277.68,'inputs':{str(p):h for p,h in inputs},'rows':rows},f,indent=2);f.write('\n')
    print(json.dumps(rows))


if __name__=='__main__':
    main()

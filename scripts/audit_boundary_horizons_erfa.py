# SPDX-License-Identifier: CPAL-1.0
"""Nine package handoffs, Horizons Q45/Q20 and ERFA long-term orientation.

python scripts/audit_boundary_horizons_erfa.py KERNEL MANIFEST EXE NEW_JSON
Network requests are read-only; inputs are never copied or modified.
"""
import csv
import io
import json
import math
from pathlib import Path
import re
import sys
import urllib.parse
import urllib.request
import erfa

from audit_horizons_ecliptic_radial_rates import AU_KM, TARGETS, four_point, sha
from audit_package_orientation_horizons import EXPECTED_SOURCE, engine, erfa_long_ecliptic, wrap


def main():
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    kernel, manifest, exe, output = map(lambda x: Path(x).resolve(), sys.argv[1:])
    if output.exists() or output in (kernel, manifest, exe):
        raise SystemExit('output exists or aliases input')
    meta = json.loads(manifest.read_text(encoding='utf8'))
    if meta.get('schema') != 'virgostars.de441.mobile-packs.v1' or len(meta['packages']) != 10 or meta['source']['sha256'] != EXPECTED_SOURCE:
        raise SystemExit('unexpected manifest')
    mh, eh = sha(manifest), sha(exe)
    if kernel.stat().st_size != meta['source']['bytes'] or sha(kernel) != EXPECTED_SOURCE:
        raise SystemExit('source mismatch')
    bounds = sorted(x['artifact']['requested_start_jd_tdb'] for x in meta['packages'])[1:]
    offsets = [-0.04, -0.02, 0.0, 0.02, 0.04]
    dates = sorted({b + h for b in bounds for h in offsets})
    tts = [r['jdTt'] for r in engine(exe, kernel, [('tt', date) for date in dates])]
    times = dict(zip(dates, tts))
    samples = engine(exe, kernel, [(i, b) for b in bounds for i in range(10)])
    responses, refs = [], {}
    for body, (_, command, name) in enumerate(TARGETS):
        params = {'format':'json','COMMAND':f"'{command}'",'OBJ_DATA':"'NO'",'MAKE_EPHEM':"'YES'",'EPHEM_TYPE':"'OBSERVER'",'CENTER':"'500@399'",'TLIST':"'"+' '.join(f'{tt:.9f}' for tt in tts)+"'",'TLIST_TYPE':"'JD'",'TIME_TYPE':"'TT'",'CAL_FORMAT':"'JD'",'QUANTITIES':"'20,23,45'",'REF_SYSTEM':"'ICRF'",'ANG_FORMAT':"'DEG'",'CSV_FORMAT':"'YES'",'EXTRA_PREC':"'YES'",'APPARENT':"'AIRLESS'"}
        url = 'https://ssd.jpl.nasa.gov/api/horizons.api?' + urllib.parse.urlencode(params)
        with urllib.request.urlopen(url, timeout=30) as response:
            raw = json.load(response)['result']
        if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}', raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}', raw) or 'Atmos refraction: NO (AIRLESS)' not in raw:
            raise SystemExit('incorrect Horizons metadata')
        header = next((line for line in raw.splitlines() if 'RA_(ICRF-a-app)' in line), '')
        columns = [x.strip() for x in header.split(',')]
        if 'Date_________JDTT' not in columns:
            raise SystemExit('wrong time scale')
        indexes = [columns.index(x) for x in ('delta','deldot','S-O-T','RA_(ICRF-a-app)','DEC_(ICRF-a-app)')]
        start, end = raw.find('$$SOE'), raw.find('$$EOE')
        if start < 0 or end <= start:
            raise SystemExit('missing Horizons table')
        rows = list(csv.reader(io.StringIO(raw[start+5:end].strip())))
        if len(rows) != len(dates):
            raise SystemExit('unexpected row count')
        for date, tt, row in zip(dates, tts, rows):
            if abs(float(row[0]) - tt) > 1e-8:
                raise SystemExit('epoch mismatch')
            values = [float(row[i]) for i in indexes]
            if not all(math.isfinite(v) for v in values):
                raise SystemExit('nonfinite reference')
            lon, lat = erfa_long_ecliptic(values[3], values[4], tt)
            refs[(body,date)] = {'lon':lon,'lat':lat,'range':values[0],'q20':values[1],'elongation':values[2]}
        responses.append({'target':name,'url':url,'raw':raw})
    records=[]
    for i,b in enumerate(bounds):
        for body in range(10):
            jd=[b+h for h in offsets]; tt=[times[t] for t in jd]; r=[refs[(body,t)] for t in jd]
            lon0=r[2]['lon']; series=[[lon0+wrap(x['lon']-lon0) for x in r],[x['lat'] for x in r],[x['range'] for x in r]]
            derivatives=[four_point(values,tt,[0,1,3,4],tt[2]) for values in series]
            v=samples[i*10+body]['candidate']
            diff=[wrap(v[0]-r[2]['lon'])*3600,(v[1]-r[2]['lat'])*3600,v[2]-r[2]['range'],(v[3]-derivatives[0])*3600,(v[4]-derivatives[1])*3600,v[5]-derivatives[2]]
            records.append({'jdTdb':b,'target':TARGETS[body][2],'diff':diff,'elongation':r[2]['elongation'],'q20MinusDerivativeKmS':r[2]['q20']-derivatives[2]*AU_KM/86400})
    limits=[1.0,1.0,1e-9,0.1,0.1,5e-10]
    summary={'cases':len(records),'maxAbs':[max(abs(r['diff'][i]) for r in records) for i in range(6)],
             'failures':[sum(abs(r['diff'][i])>limits[i] for r in records) for i in range(6)],
             'q20Failures':sum(abs(r['q20MinusDerivativeKmS'])>0.001 for r in records),
             'q20MaxAbsKmS':max(abs(r['q20MinusDerivativeKmS']) for r in records),
             'minimumNonSunElongation':min(r['elongation'] for r in records if r['target']!='Sun')}
    if sha(kernel)!=EXPECTED_SOURCE or sha(manifest)!=mh or sha(exe)!=eh:
        raise SystemExit('input changed')
    result={'schema':'multipack-boundary-horizons/1','sourceSha256':EXPECTED_SOURCE,'manifestSha256':mh,'binarySha256':eh,'erfaVersion':erfa.__version__,'limits':limits,'summary':summary,'records':records,'responses':responses}
    with output.open('x',encoding='utf8',newline='\n') as out:
        json.dump(result,out,indent=2,allow_nan=False); out.write('\n')
    print(json.dumps(summary))
    if any(summary['failures']) or summary['q20Failures']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

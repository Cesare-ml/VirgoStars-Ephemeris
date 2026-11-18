# SPDX-License-Identifier: CPAL-1.0
"""Owner-local DE441 UT1 scenarios and ERFA/Horizons reference audit.

python scripts/audit_historical_ut1_provider.py KERNEL MANIFEST PACK_DIR EXE NEW_DIR
Creates only reports and a small corrupt synthetic file in NEW_DIR; never copies kernels.
"""
import csv
import io
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import urllib.parse
import urllib.request
import erfa
from audit_horizons_ecliptic_radial_rates import TARGETS, sha
from audit_package_orientation_horizons import EXPECTED_SOURCE, erfa_long_ecliptic, wrap


def main():
    if len(sys.argv)!=6: raise SystemExit(__doc__)
    kernel,manifest,packs,exe,out=map(lambda x:Path(x).resolve(),sys.argv[1:])
    if out.exists() or any(out==p or out in p.parents for p in (kernel,manifest,packs,exe)) or packs in out.parents:
        raise SystemExit('output directory must be new and separate from inputs')
    meta=json.loads(manifest.read_text(encoding='utf8'))
    if meta.get('schema')!='virgostars.de441.mobile-packs.v1' or len(meta['packages'])!=10 or meta['source']['sha256']!=EXPECTED_SOURCE: raise SystemExit('unexpected manifest')
    inputs=[(kernel,EXPECTED_SOURCE,meta['source']['bytes']),(manifest,sha(manifest),manifest.stat().st_size),(exe,sha(exe),exe.stat().st_size)]
    for package in meta['packages']:
        art=package['artifact']
        if Path(art['file']).name!=art['file']: raise SystemExit('invalid filename')
        inputs.append((packs/art['file'],art['sha256'],art['bytes']))
    def verify():
        for path,digest,size in inputs:
            if path.stat().st_size!=size or sha(path)!=digest: raise RuntimeError(f'input mismatch: {path}')
    verify()
    dates=[]
    for year in range(0,2301,25):
        for month in (1,4,7,10):
            a,b=erfa.cal2jd(year,month,1); dates.append(('seasonal',year,month,float(a+b+0.5)))
    for year in range(0,2301):
        for month in range(1,13):
            a,b=erfa.cal2jd(year,month,15); dates.append(('monthly',year,month,float(a+b+0.5)))
    boundaries=sorted({p['artifact'][key] for p in meta['packages'] for key in ('requested_start_jd_tdb','requested_end_exclusive_jd_tdb')})
    for boundary in boundaries:
        for offset in (-1.0,0.0,1.0):
            dates.append(('boundary',None,None,float(boundary+(offset-64.0)/86400.0)))
    out.mkdir(parents=True,exist_ok=False)
    (out/'corrupt-test-only.eph').write_bytes(b'not a DE441 file')
    input_text=''.join(f'{jd:.17f},64.0\n' for _,_,_,jd in dates)
    run=subprocess.run([str(exe),str(kernel),str(packs),str(out/'absent.eph'),str(out/'corrupt-test-only.eph')],input=input_text,text=True,capture_output=True,check=False)
    (out/'stdout.csv').write_text(run.stdout,encoding='utf8')
    (out/'stderr.txt').write_text(run.stderr,encoding='utf8')
    if run.returncode: raise SystemExit(f'Rust audit failed: {run.returncode}; see stderr.txt')
    cases=[]; times=[]; summaries=[]
    for line in run.stdout.splitlines():
        f=line.split(',')
        if f[0]=='case':
            cases.append({'pack':f[1],'body':int(f[2]),'sample':f[3],'ut1':float(f[4]),'tt':float(f[5]),'tdb':float(f[6]),'deltaT':float(f[7]),'values':list(map(float,f[8:14]))})
        elif f[0]=='time': times.append(list(map(float,f[1:])))
        elif f[0]=='summary': summaries.append(line)
        else: raise SystemExit('unexpected Rust output')
    if len(cases)!=300 or len(times)!=len(dates) or len(summaries)!=1: raise SystemExit('wrong Rust output count')
    residuals=[]
    for (kind,year,month,request),(ut1,tt,tdb,model_seconds) in zip(dates,times):
        if abs(request-ut1)>1e-9: raise SystemExit('time epoch mismatch')
        erfa_seconds=float(erfa.dtdb(2451545.0,tt-2451545.0,(ut1+0.5)%1,0.0,0.0,0.0))
        difference=(tdb-tt)*86400.0-erfa_seconds
        residuals.append({'kind':kind,'year':year,'month':month,'ut1':ut1,'tt':tt,'deltaSeconds':difference,'erfaSeconds':erfa_seconds,
                          'modelMinusErfaSeconds':model_seconds-erfa_seconds,
                          'representationMinusModelSeconds':(tdb-tt)*86400.0-model_seconds})
    refs=[]; records=[]
    for body,(_,command,name) in enumerate(TARGETS):
        group=[x for x in cases if x['body']==body]
        tts=[float(f"{x['tt']:.9f}") for x in group]
        params={'format':'json','COMMAND':f"'{command}'",'OBJ_DATA':"'NO'",'MAKE_EPHEM':"'YES'",'EPHEM_TYPE':"'OBSERVER'",'CENTER':"'500@399'",'TLIST':"'"+' '.join(f'{x:.9f}' for x in tts)+"'",'TLIST_TYPE':"'JD'",'TIME_TYPE':"'TT'",'CAL_FORMAT':"'JD'",'QUANTITIES':"'20,23,45'",'REF_SYSTEM':"'ICRF'",'ANG_FORMAT':"'DEG'",'CSV_FORMAT':"'YES'",'EXTRA_PREC':"'YES'",'APPARENT':"'AIRLESS'"}
        url='https://ssd.jpl.nasa.gov/api/horizons.api?'+urllib.parse.urlencode(params)
        with urllib.request.urlopen(url,timeout=30) as response: raw=json.load(response)['result']
        if not re.search(rf'Target body name: {re.escape(name)} \({command}\).*\{{source: DE441\}}',raw) or not re.search(r'Center body name: Earth \(399\).*\{source: DE441\}',raw) or 'Atmos refraction: NO (AIRLESS)' not in raw: raise SystemExit('incorrect Horizons metadata')
        header=next((x for x in raw.splitlines() if 'RA_(ICRF-a-app)' in x),'')
        columns=[x.strip() for x in header.split(',')]
        if 'Date_________JDTT' not in columns: raise SystemExit('incorrect TT reference')
        indexes=[columns.index(x) for x in ('delta','S-O-T','RA_(ICRF-a-app)','DEC_(ICRF-a-app)')]
        start,end=raw.find('$$SOE'),raw.find('$$EOE')
        if start<0 or end<=start: raise SystemExit('missing reference table')
        rows=list(csv.reader(io.StringIO(raw[start+5:end].strip())))
        if len(rows)!=len(group): raise SystemExit('reference row count')
        for case,tt,row in zip(group,tts,rows):
            if abs(float(row[0])-tt)>1e-8: raise SystemExit('reference epoch mismatch')
            v=[float(row[i]) for i in indexes]
            if not all(math.isfinite(x) for x in v): raise SystemExit('nonfinite reference')
            lon,lat=erfa_long_ecliptic(v[2],v[3],tt); obs=case['values']
            records.append({'pack':case['pack'],'body':name,'sample':case['sample'],'tt':tt,'elongationDeg':v[1],
                            'lonArcsec':wrap(obs[0]-lon)*3600,'latArcsec':(obs[1]-lat)*3600,'rangeAu':obs[2]-v[0]})
        refs.append({'target':name,'url':url,'raw':raw})
    verify()
    def metric(key,limit):
        worst=max(records,key=lambda x:abs(x[key])); return {'maxAbs':abs(worst[key]),'failures':sum(abs(x[key])>limit for x in records),'limit':limit,'worst':worst}
    worst=max(residuals,key=lambda x:abs(x['deltaSeconds']))
    cohort_summary={}
    for kind in ('seasonal','monthly','boundary'):
        subset=[x for x in residuals if x['kind']==kind]
        cohort_summary[kind]={'count':len(subset),'modelMaxSeconds':max(abs(x['modelMinusErfaSeconds']) for x in subset),
                              'representedMaxSeconds':max(abs(x['deltaSeconds']) for x in subset),
                              'modelFailures':sum(abs(x['modelMinusErfaSeconds'])>1e-9 for x in subset),
                              'representedFailures':sum(abs(x['deltaSeconds'])>25e-6 for x in subset)}
    summary={'providerCases':len(cases),'rustSummary':summaries[0],'cohorts':cohort_summary,
             'erfaSamples':len(residuals),'erfaMaxAbsSeconds':abs(worst['deltaSeconds']),'erfaWorst':worst,'erfaLimitSeconds':0.0001,
             'erfaFailures':sum(abs(x['deltaSeconds'])>0.0001 for x in residuals),
             'posthocModelMaxAbsSeconds':max(abs(x['modelMinusErfaSeconds']) for x in residuals),
             'posthocRepresentationMaxAbsSeconds':max(abs(x['representationMinusModelSeconds']) for x in residuals),'jplCases':len(records),
             'longitude':metric('lonArcsec',1.0),'latitude':metric('latArcsec',1.0),'distance':metric('rangeAu',1e-9)}
    result={'schema':'de441-historical-ut1-provider/2','status':'EXPERIMENTAL','erfaVersion':erfa.__version__,'sourceSha256':EXPECTED_SOURCE,'manifestSha256':inputs[1][1],'binarySha256':inputs[2][1],
            'summary':summary,'cases':cases,'erfaResiduals':residuals,'records':records,'jplResponses':refs}
    with (out/'report.json').open('x',encoding='utf8',newline='\n') as file: json.dump(result,file,indent=2,allow_nan=False);file.write('\n')
    print(json.dumps(summary))
    if summary['erfaFailures'] or any(v['modelFailures'] or v['representedFailures'] for v in cohort_summary.values()) or any(summary[k]['failures'] for k in ('longitude','latitude','distance')): raise SystemExit(1)


if __name__=='__main__': main()

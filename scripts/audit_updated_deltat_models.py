# SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""Evaluate caller-supplied S5 spline and existing USNO reader against references.

python scripts/audit_updated_deltat_models.py EXE S5_ROWS USNO_DIR FINALS PRIOR_REPORT NEW_REPORT
Uses externally supplied source snapshots, no network and no model duplicated here.
"""
import datetime as dt
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import warnings
import erfa


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024*1024), b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    if len(sys.argv) != 7:
        raise SystemExit(__doc__)
    exe, s5, usno, finals, prior, out = map(lambda s: Path(s).resolve(), sys.argv[1:])
    if out.exists() or out in (exe, s5, finals, prior) or usno in out.parents:
        raise SystemExit('output must be new and separate from inputs')
    sources = [exe, s5, finals, prior] + [usno / n for n in ('deltat.data', 'historic_deltat.data', 'deltat.preds')]
    before = {str(p): sha(p) for p in sources}
    if before[str(s5)] != 'bd953d93b5f2b816c9b699ceb5572d2cc82b624009880d261a2b2ef5a1c7050c':
        raise ValueError('unexpected extracted S5 source hash')
    previous = json.loads(prior.read_text(encoding='utf8'))
    if previous.get('schema') != 'de441-deltat-reference/1':
        raise ValueError('unexpected previous report')
    events = previous['events']
    historic = previous['historic']
    jpl = previous['horizons']
    # Published S6 rounding applies; ambiguous coarse 1880 and 1900 signs
    # are documented and intentionally not used as positive conformance checks.
    fixtures = [(0.0,10440.0,5.0),(500.0,5590.0,5.0),(1000.0,1650.0,5.0),
                (1500.0,290.0,5.0),(1600.0,110.0,5.0),(1700.0,14.0,0.5),
                (1800.0,18.37,0.005),(1900.0,-1.98,0.005),(2000.0,63.81,0.005),
                (2010.0,66.06,0.005),(2019.0,69.24,0.005)]
    knots = [float(line.split()[1]) for line in s5.read_text().splitlines()[1:]]
    if len(knots)!=57:
        raise ValueError('wrong spline knot count')
    daily = []
    for line in finals.read_text(encoding='ascii').splitlines():
        if len(line) < 78 or line[57:58] != 'I':
            continue
        try:
            mjd, dut1, error = float(line[7:15]), float(line[58:68]), float(line[68:78])
            yr, mo, day, _ = erfa.jd2cal(2400000.5, mjd)
            yr,mo,day = int(yr),int(mo),int(day)
            date = dt.date(yr,mo,day)
        except (ValueError, OverflowError):
            continue
        if day != 15 or date < dt.date(1973, 2, 1) or date >= dt.date(2026, 10, 1):
            continue
        with warnings.catch_warnings():
            warnings.simplefilter('ignore', erfa.ErfaWarning)
            tai_utc = float(erfa.dat(yr,mo,day,0.0))
        if yr >= 2017 and tai_utc != 37.0:
            raise ValueError('leap seconds disagree with IERS Bulletin C 72')
        daily.append({'date': date.isoformat(), 'jd': 2400000.5+mjd,
                      'dut1': dut1, 'eopError': error, 'taiMinusUtc': tai_utc,
                      'reference': 32.184+tai_utc-dut1})
    if len(daily) < 600 or len(historic) != 630 or len(jpl) != 22:
        raise ValueError('source coverage mismatch')
    queries = [('S',float(x[0])) for x in fixtures]
    queries += [('S',float(x['decimalYear'])) for x in events]
    queries += [('S',float(x['year'])) for x in historic]
    queries += [('S',float(x['decimalYear'])) for x in jpl]
    queries += [('U',float(x['jd'])) for x in daily]
    queries += [('S',y) for y in (2019.01, 2100.0, 2300.0)]
    queries += [('S',y) for knot in knots for y in (knot-1e-9,knot)]
    first_month_jd=float(sum(erfa.cal2jd(1973,2,1)))
    queries += [('S',1973.0+31.0/365.0),('U',first_month_jd),('U',2461315.5),('U',2461406.5)]
    run = subprocess.run([str(exe),str(s5),str(usno/'deltat.data'),str(usno/'historic_deltat.data'),str(usno/'deltat.preds')],
                         input=''.join(f'{k},{x:.12f}\n' for k,x in queries),text=True,capture_output=True,check=True)
    lines=run.stdout.splitlines()
    if len(lines) != len(queries)+1 or not lines[0].startswith('meta,'):
        raise ValueError('unexpected Rust response')
    meta=lines[0].split(',')
    outputs=[]
    for (kind,n),line in zip(queries,lines[1:]):
        row=line.split(',')
        if row[0]!=kind or abs(float(row[1])-n)>1e-8:
            raise ValueError('Rust response mismatch')
        outputs.append(row[2:])
    stream=iter(outputs)
    result_fixtures=[]
    for year,reference,limit in fixtures:
        actual=float(next(stream)[0]); result_fixtures.append({'year':year,'reference':reference,'limit':limit,'model':actual,'residual':actual-reference})
    results_events=[]
    for e in events:
        value=float(next(stream)[0]); results_events.append({'year':e['year'],'month':e['month'],'model':value,
             'intervals':e['intervals'],'inPublishedInterval':any(a <= value <= b for a,b in e['intervals'])})
    historical=[]
    for e in historic:
        value=float(next(stream)[0]); historical.append({'year':e['year'],'model':value,'reference':e['reference'],
            'referenceError':e['referenceError'],'residual':value-e['reference']})
    jpl_rows=[]
    for e in jpl:
        value=float(next(stream)[0]); jpl_rows.append({'year':e['decimalYear'],'model':value,'reference':e['reference'],'residual':value-e['reference']})
    for e in daily:
        fields=next(stream)
        if len(fields)!=3 or fields[2]!='Measured' or fields[1]!='unknown':
            raise ValueError('unexpected USNO observation kind or uncertainty')
        e['model']=float(fields[0]);e['residual']=e['model']-e['reference']
    outside=[next(stream)[0] for _ in range(3)]
    if outside != ['ERR','ERR','ERR']:
        raise ValueError('spline extrapolated beyond published coverage')
    knot_rows=[]
    for knot in knots:
        left=float(next(stream)[0]); right=float(next(stream)[0])
        knot_rows.append({'year':knot,'leftNear':left,'atKnot':right,'difference':right-left})
    join_spline=float(next(stream)[0]); join_table=next(stream)
    after_observation=next(stream); first_future=next(stream)
    if after_observation[1:] != ['unknown','Predicted'] or first_future[-1] != 'Predicted':
        raise ValueError('unexpected actual observation/prediction uncertainty')
    if before != {str(p):sha(p) for p in sources}:
        raise ValueError('source changed')
    def maximum(rows):
        return max(rows,key=lambda r:abs(r['residual']))
    eligible=[x for x in historical if x['referenceError']>0]
    summary={'s6':{'count':len(result_fixtures),'failures':sum(abs(x['residual'])>x['limit'] for x in result_fixtures)},
             'events':results_events,'historic':{'count':len(historical),'outsideReportedError':sum(abs(x['residual'])>x['referenceError'] for x in eligible),'worst':maximum(historical)},
             'horizons':{'count':len(jpl_rows),'worst':maximum(jpl_rows)},
             'daily':{'count':len(daily),'threshold':0.05,'failures':sum(abs(x['residual'])>0.05 for x in daily),'worst':maximum(daily),'first':daily[0]['date'],'last':daily[-1]['date']},
             'knots':{'count':len(knot_rows),'threshold':0.005,'failures':sum(abs(x['difference'])>0.005 for x in knot_rows),
                      'worst':max(knot_rows,key=lambda x:abs(x['difference']))},
             'splice1973':{'spline':join_spline,'monthly':float(join_table[0]),'monthlyMinusSpline':float(join_table[0])-join_spline},
             'afterLastObservation':after_observation,'first2027Prediction':first_future,
             'lastMeasuredJd':float(meta[1]),'lastPredictionJd':float(meta[2]),'outsidePublishedSpline':outside}
    result={'schema':'de441-updated-deltat-audit/1','status':'EXPERIMENTAL','sourceHashes':before,
            'summary':summary,'s6':result_fixtures,'events':results_events,'historic':historical,'horizons':jpl_rows,'daily':daily,'knots':knot_rows}
    with out.open('x',encoding='utf8',newline='\n') as f:
        json.dump(result,f,indent=2,allow_nan=False);f.write('\n')
    print(json.dumps(summary))
    if summary['s6']['failures'] or summary['daily']['failures'] or summary['knots']['failures'] or not all(x['inPublishedInterval'] for x in results_events):
        raise SystemExit(1)


if __name__ == '__main__':
    main()

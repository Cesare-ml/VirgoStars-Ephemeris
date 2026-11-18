# SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
"""Extract 58 S5 coefficient rows from the supplied official 2021 PDF.

python scripts/extract_morrison_s5.py SUPPLEMENT.pdf NEW_ROWS.txt
Uses pypdf and never downloads or redistributes the source document.
"""
import hashlib
from pathlib import Path
import re
import sys
from pypdf import PdfReader

EXPECTED_PDF = 'e30601b2c9ac2169f923fdaa6c9bc76628b393b36ad3a0e18385111166e24255'


def main():
    if len(sys.argv)!=3:
        raise SystemExit(__doc__)
    pdf, out = map(lambda s: Path(s).resolve(),sys.argv[1:])
    if out.exists() or out==pdf:
        raise SystemExit('output must be new')
    before=hashlib.sha256(pdf.read_bytes()).hexdigest()
    if before != EXPECTED_PDF:
        raise ValueError('unexpected supplement PDF hash')
    text=PdfReader(str(pdf)).pages[34].extract_text(extraction_mode='layout').replace('−','-')
    rows=[]
    for line in text.splitlines():
        fields=line.split()
        if len(fields)==7 and re.fullmatch(r'\d+',fields[0]) and int(fields[0])==len(rows)+1:
            [float(v) for v in fields[1:]]
            rows.append(fields)
    if len(rows)!=58 or rows[0][1]!='-720.0' or rows[-1][2]!='2019.0':
        raise ValueError('unexpected S5 table')
    if hashlib.sha256(pdf.read_bytes()).hexdigest()!=before:
        raise ValueError('PDF changed')
    with out.open('x',encoding='utf8',newline='\n') as file:
        file.writelines(' '.join(row)+'\n' for row in rows)
    print('rows=58 sha256='+hashlib.sha256(out.read_bytes()).hexdigest())


if __name__=='__main__':
    main()

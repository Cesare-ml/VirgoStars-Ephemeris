"""Generate optional ERFA 2.0.1.5 long-term precession reference locally.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
No network calls, no data bundled. This compares implementations of the same
Vondrak model, not the physical accuracy of that model.
"""
import csv
import sys
import erfa

JDS = (1721242, 2086484, 2451727, 2816969, 3182212, 3547454)

def main():
    if len(sys.argv) != 2:
        raise ValueError("usage: generate_long_term_erfa_reference.py <new-output.csv>")
    if erfa.__version__ != "2.0.1.5":
        raise ValueError("reference requires pyerfa 2.0.1.5")
    with open(sys.argv[1], "x", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["jd_tt"] + [f"eq{i}{j}" for i in range(3) for j in range(3)] + [f"ecl{i}" for i in range(3)])
        for jd in JDS:
            epoch = 2000.0 + (jd - 2451545.0) / 365.25
            values = list(erfa.ltp(epoch).flat) + list(erfa.ltpecl(epoch))
            writer.writerow([jd] + [format(value, ".17g") for value in values])
    print(f"ERFA {erfa.__version__}: {len(JDS)} mean-axis reference epochs")

if __name__ == "__main__":
    main()

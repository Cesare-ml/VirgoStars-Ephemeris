"""Local comparison data from ERFA 2000A; neither model is physical truth.

SPDX-License-Identifier: CPAL-1.0. Powered by VirgoStars; see LICENSE.
No network calls. The reference is not bundled.
"""
import csv
import math
import sys
import erfa

def main():
    if len(sys.argv) != 2 or erfa.__version__ != "2.0.1.5":
        raise ValueError("usage: pyerfa 2.0.1.5 and <new-output.csv> required")
    with open(sys.argv[1], "x", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["jd_tt", "dpsi_arcsec"])
        for year in range(0, 5001, 50):
            offset = (year - 2000) * 365.25
            dpsi, _ = erfa.nut00a(2451545.0, offset)
            writer.writerow([format(2451545.0 + offset, ".17g"), format(dpsi * (180.0 * 3600.0 / math.pi), ".17g")])
    print("ERFA 2.0.1.5 2000A nutation: 101 sampled epochs")

if __name__ == "__main__":
    main()

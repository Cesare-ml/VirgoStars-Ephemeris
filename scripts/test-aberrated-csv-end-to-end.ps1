# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# 
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# 
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# 
# The Original Developer is the Initial Developer.
# 
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# 
# Contributors retain applicable rights in their own contributions under the terms of this License.
# 
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

# Offline end-to-end smoke test: real Cargo executables + a tiny FICTIONAL
# classic DE441 file. No NASA/JPL bytes, external data or networking.
# Designed for PowerShell 7 on Windows, with a local Rust toolchain.
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

function Put-Double {
    param([byte[]]$Bytes, [int]$Offset, [double]$Value)
    [byte[]]$encoded = [BitConverter]::GetBytes($Value)
    if (-not [BitConverter]::IsLittleEndian) { [Array]::Reverse($encoded) }
    [Array]::Copy($encoded, 0, $Bytes, $Offset, 8)
}

function Put-Int32 {
    param([byte[]]$Bytes, [int]$Offset, [int]$Value)
    [byte[]]$encoded = [BitConverter]::GetBytes($Value)
    if (-not [BitConverter]::IsLittleEndian) { [Array]::Reverse($encoded) }
    [Array]::Copy($encoded, 0, $Bytes, $Offset, 4)
}

function Put-Series {
    param(
        [byte[]]$Bytes, [int]$FirstWord,
        [double[]]$XAxis, [double[]]$YAxis, [double[]]$ZAxis
    )
    [double[][]]$coords = @($XAxis, $YAxis, $ZAxis)
    for ($axis = 0; $axis -lt 3; $axis++) {
        for ($term = 0; $term -lt 3; $term++) {
            [int]$word = $FirstWord - 1 + $axis * 3 + $term
            Put-Double $Bytes (6000 + $word * 8) $coords[$axis][$term]
        }
    }
}

function New-FictionalDe441 {
    param([string]$Destination)
    # Duplicates fixture values from tests/civil_aberrated_batch.rs.
    # This is synthetic, NEVER an official NASA/JPL kernel.
    [byte[]]$bytes = [byte[]]::new(9000)
    [byte[]]$title = [Text.Encoding]::ASCII.GetBytes('JPL Planetary Ephemeris DE441')
    [Array]::Copy($title, 0, $bytes, 0, $title.Length)
    Put-Double $bytes 2652 2461344.0
    Put-Double $bytes 2660 2461376.0
    Put-Double $bytes 2668 32.0
    Put-Double $bytes 2688 81.0
    Put-Int32 $bytes 2840 441
    foreach ($pointer in @(
        @{ Index = 0; First = 3; Coeff = 3 },
        @{ Index = 2; First = 12; Coeff = 3 },
        @{ Index = 9; First = 21; Coeff = 3 },
        @{ Index = 12; First = 373; Coeff = 1 }
    )) {
        [int]$offset = if ($pointer.Index -eq 12) {
            2844
        } else {
            2696 + $pointer.Index * 12
        }
        Put-Int32 $bytes $offset $pointer.First
        Put-Int32 $bytes ($offset + 4) $pointer.Coeff
        Put-Int32 $bytes ($offset + 8) 1
    }
    Put-Double $bytes 6000 2461344.0
    Put-Double $bytes 6008 2461376.0
    Put-Series $bytes 3 ([double[]]@(200000000.0, 1440000.0, 10000.0)) ([double[]]@(70000000.0, -20800000.0, -30000.0)) ([double[]]@(15000000.0, 4000000.0, 5000.0))
    Put-Series $bytes 12 ([double[]]@(1000000.0, 800000.0, 5000.0)) ([double[]]@(300000.0, 17600000.0, 200000.0)) ([double[]]@(0.0, 3200000.0, 1000.0))
    Put-Series $bytes 21 ([double[]]@(10000.0, 800.0, 20.0)) ([double[]]@(20000.0, 2400.0, 10.0)) ([double[]]@(1000.0, 160.0, 1.0))
    [IO.File]::WriteAllBytes($Destination, $bytes)
}

function Run-Cargo {
    param([string[]]$Arguments)
    [string[]]$outputLines = @(& cargo @Arguments)
    return [pscustomobject]@{
        ExitCode = [int]$LASTEXITCODE
        Lines = $outputLines
    }
}

function Save-Csv {
    param([string]$Destination, [string[]]$Lines)
    [IO.File]::WriteAllText(
        $Destination,
        ($Lines -join [Environment]::NewLine) + [Environment]::NewLine,
        [Text.UTF8Encoding]::new($false)
    )
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$temp = Join-Path ([IO.Path]::GetTempPath()) ('virgostars-csv-smoke-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp -ErrorAction Stop | Out-Null
Push-Location $root
try {
    $kernel = Join-Path $temp 'synthetic-classic-de441.eph'
    $civilCsv = Join-Path $temp 'civil.csv'
    $utcCsv = Join-Path $temp 'utc.csv'
    $changedCsv = Join-Path $temp 'modified-speed.csv'
    $invalidFrameCsv = Join-Path $temp 'wrong-frame.csv'
    New-FictionalDe441 $kernel

    $common = @('run', '--quiet', '--locked', '-p', 'virgostars-ephemeris-core', '--example')
    $targets = @('Moon', 'Mercury', 'Moon')
    $civil = Run-Cargo ($common + @('aberrated_csv', '--', $kernel, '2026-11-10T13:30:00+01:00') + $targets)
    if ($civil.ExitCode -ne 0) { throw "civil CSV exporter exited $($civil.ExitCode)" }
    if ($civil.Lines.Count -ne 4) { throw 'civil export must contain one header and three data rows' }
    Save-Csv $civilCsv $civil.Lines

    $utc = Run-Cargo ($common + @('aberrated_csv', '--', $kernel, '2026-11-10T12:30:00Z') + $targets)
    if ($utc.ExitCode -ne 0) { throw "UTC CSV exporter exited $($utc.ExitCode)" }
    if (($civil.Lines -join [Environment]::NewLine) -cne ($utc.Lines -join [Environment]::NewLine)) {
        throw 'equivalent civil UTC offsets did not produce identical complete output'
    }
    Save-Csv $utcCsv $utc.Lines

    $compareBase = $common + @('compare_aberrated_csv', '--')
    $zeroTolerance = @('0', '0', '0', '0', '0', '0')
    $same = Run-Cargo ($compareBase + @($civilCsv, $utcCsv) + $zeroTolerance)
    if ($same.ExitCode -ne 0) { throw "exact CSV comparison exited $($same.ExitCode)" }
    if ($same.Lines.Count -ne 4 -or @($same.Lines | Where-Object { $_ -match ',false$' }).Count -ne 0) {
        throw 'identical output must produce one header and three passing residual rows'
    }
    Write-Host '[PASS] Civil UTC offsets, duplicate targets and exact end-to-end CSV comparison'

    # Change exactly one signed angular velocity without changing metadata.
    [string[]]$changedRows = @($civil.Lines)
    [string[]]$cells = $changedRows[1].Split(',')
    [double]$rate = [double]::Parse($cells[8], [Globalization.CultureInfo]::InvariantCulture)
    $cells[8] = ($rate + 0.25).ToString('R', [Globalization.CultureInfo]::InvariantCulture)
    $changedRows[1] = $cells -join ','
    Save-Csv $changedCsv $changedRows
    $mismatch = Run-Cargo ($compareBase + @($changedCsv, $utcCsv) + $zeroTolerance)
    if ($mismatch.ExitCode -eq 0) { throw 'modified signed velocity was incorrectly accepted' }
    if ($mismatch.Lines.Count -ne 4 -or $mismatch.Lines[1] -notmatch ',false$') {
        throw 'numerical mismatch must generate a complete report and a failing result'
    }
    Write-Host '[PASS] Changed signed velocity fails configured zero tolerance'

    # An incompatible reference frame must never become a numeric report.
    [string[]]$frameRows = @($civil.Lines | ForEach-Object {
        $_.Replace('fixed_ICRF_equatorial', 'ecliptic_of_date')
    })
    Save-Csv $invalidFrameCsv $frameRows
    $wrongFrame = Run-Cargo ($compareBase + @($civilCsv, $invalidFrameCsv) + $zeroTolerance)
    if ($wrongFrame.ExitCode -eq 0 -or $wrongFrame.Lines.Count -ne 0) {
        throw 'mismatched scientific frame must fail without partial CSV output'
    }
    Write-Host '[PASS] Mismatched frame rejected before comparison'
}
finally {
    Pop-Location
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}

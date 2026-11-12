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

# User-initiated, offline real-kernel validation receipt for classic DE441.
# The program NEVER downloads or redistributes an ephemeris binary.
# Real scientific tests run ONLY when a separately acquired file is supplied.
[CmdletBinding()]
param(
    [string]$De441Path,
    [string]$OutputRoot,
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

# Reject tiny synthetic fixtures and SPK kernels before any misleading
# "real DE441" pass claim. The Rust DE441 reader is still authoritative
# for checking the classic ephemeris binary header and contents.
function Assert-RealKernelCandidate {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not [IO.File]::Exists($Path)) {
        throw 'A readable, existing classic DE441 file is required (-De441Path).'
    }
    $file = Get-Item -LiteralPath $Path -ErrorAction Stop
    if ($file.Length -lt 1000000000) {
        throw 'The proposed file is under 1 GB. Rejecting synthetic/truncated data as a real classic DE441 kernel.'
    }
    $stream = [IO.File]::OpenRead($file.FullName)
    try {
        [byte[]]$prefix = [byte[]]::new(16)
        if ($stream.Read($prefix, 0, $prefix.Length) -ne $prefix.Length) {
            throw 'Kernel header cannot be read.'
        }
        if ([Text.Encoding]::ASCII.GetString($prefix).StartsWith('DAF/SPK')) {
            throw 'This is a NAIF SPK/DAF file, not a supported classic DE441 .eph binary.'
        }
    }
    finally {
        $stream.Dispose()
    }
    return $file
}

# Must have both concrete tests PASS and an explicit two-test summary.
# This detects accidentally compiling without executing #[ignore] tests.
function Test-GatePass {
    param([string[]]$Lines, [int]$ExitCode, [string[]]$ExpectedTests)
    if ($ExitCode -ne 0) { return $false }
    $body = ($Lines -join [Environment]::NewLine)
    foreach ($name in $ExpectedTests) {
        $pattern = '(?m)^test ' + [regex]::Escape($name) + ' \.\.\. ok\s*$'
        if (-not [regex]::IsMatch($body, $pattern)) { return $false }
    }
    return [regex]::IsMatch(
        $body,
        '(?m)^test result: ok\. 2 passed; 0 failed; 0 ignored;'
    )
}

function Invoke-CargoGate {
    param(
        [string]$Suite,
        [string[]]$ExpectedTests,
        [string]$Directory
    )
    [string[]]$args = @(
        'test', '--workspace', '--locked',
        '--test', $Suite, '--', '--ignored', '--nocapture'
    )
    [string[]]$lines = @(& cargo @args 2>&1 | ForEach-Object { "$_" })
    [int]$code = $LASTEXITCODE
    $logName = ($Suite + '.log')
    [IO.File]::WriteAllLines(
        (Join-Path $Directory $logName),
        $lines,
        [Text.UTF8Encoding]::new($false)
    )
    $pass = Test-GatePass -Lines $lines -ExitCode $code -ExpectedTests $ExpectedTests
    return [pscustomobject]@{
        suite = $Suite
        command = ('cargo ' + ($args -join ' '))
        expected_tests = $ExpectedTests
        exit_code = $code
        status = $(if ($pass) { 'PASS' } else { 'FAIL' })
        log = $logName
    }
}

function Invoke-PreflightSelfTest {
    [string]$temp = Join-Path ([IO.Path]::GetTempPath()) ('virgostars-de441-preflight-' + [guid]::NewGuid().ToString('N'))
    New-Item -Path $temp -ItemType Directory -ErrorAction Stop | Out-Null
    try {
        $missing = Join-Path $temp 'missing.eph'
        $small = Join-Path $temp 'fictional.eph'
        [IO.File]::WriteAllBytes($small, [byte[]]::new(9000))
        foreach ($candidate in @($missing, $small)) {
            $rejected = $false
            try {
                $null = Assert-RealKernelCandidate -Path $candidate
            }
            catch {
                $rejected = $true
            }
            if (-not $rejected) {
                throw "Preflight accepted a missing or fictional file: $candidate"
            }
        }

        $tests = @('real_first', 'real_second')
        $valid = @(
            'running 2 tests',
            'test real_first ... ok',
            'test real_second ... ok',
            'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured'
        )
        if (-not (Test-GatePass -Lines $valid -ExitCode 0 -ExpectedTests $tests)) {
            throw 'Receipt parser rejected two explicitly passing test cases.'
        }
        foreach ($invalid in @(
            @('running 0 tests', 'test result: ok. 0 passed; 0 failed; 2 ignored;'),
            @('running 2 tests', 'test real_first ... ok', 'test result: ok. 1 passed; 0 failed;'),
            @('running 2 tests', 'test real_first ... ok', 'test real_second ... FAILED', 'test result: FAILED.')
        )) {
            if (Test-GatePass -Lines $invalid -ExitCode 0 -ExpectedTests $tests) {
                throw 'Receipt parser accepted a skipped/partial/failed suite.'
            }
        }
        if (Test-GatePass -Lines $valid -ExitCode 101 -ExpectedTests $tests) {
            throw 'Receipt parser ignored a nonzero Cargo exit code.'
        }
        Write-Host '[PASS] Offline preflight rejects tiny/missing kernels and nonexecuted/failed test suites.'
    }
    finally {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($SelfTest) {
    Invoke-PreflightSelfTest
    exit 0
}

$timestamp = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
$identifier = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $HOME 'VirgoStars-Validation'
}
$reportDirectory = Join-Path ([IO.Path]::GetFullPath($OutputRoot)) $identifier
New-Item -ItemType Directory -Path $reportDirectory -ErrorAction Stop | Out-Null

$receipt = [ordered]@{
    schema_version = 1
    status = 'NOT_RUN'
    run_timestamp_utc = $timestamp
    tool = 'VirgoStars-Ephemeris real classic-DE441 receipt v1'
    repository = 'https://github.com/Cesare-ml/VirgoStars-Ephemeris'
    source_commit = $null
    file = [ordered]@{
        filename = $null
        bytes = $null
        sha256 = $null
        content_released_or_copied = $false
    }
    reference = [ordered]@{
        provider = 'NASA/JPL Horizons'
        pinned_query_date = '2026-11-10'
        tdb_jd = 2461354.5
        frame = 'ICRF'
        center = 'Earth geocenter (500@399)'
        correction = 'VEC_CORR=LT (one-way light-time ONLY)'
        targets = @('Moon', 'Venus', 'MarsBarycenter', 'JupiterBarycenter')
        caveat = 'Inverse SR boost validates recovered LT geometry, NOT independent annual-aberration law, Horizons LT+S or Swiss apparent-of-date parity.'
    }
    rustc_version = $null
    cargo_version = $null
    gates = @()
    error = $null
}
$failure = $null
$priorDe441Path = [Environment]::GetEnvironmentVariable('DE441_PATH', 'Process')
$sourceRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Push-Location $sourceRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Cargo is unavailable. Install Rust toolchain before running scientific gates.'
    }
    if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
        throw 'Rustc is unavailable. Install Rust toolchain before running scientific gates.'
    }

    $file = Assert-RealKernelCandidate -Path $De441Path
    $receipt.file.filename = $file.Name
    $receipt.file.bytes = [long]$file.Length
    $receipt.file.sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $receipt.rustc_version = ((& rustc --version) -join '')
    $receipt.cargo_version = ((& cargo --version) -join '')
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $rev = @(& git rev-parse HEAD 2>$null)
        if ($LASTEXITCODE -eq 0 -and $rev.Count -eq 1) {
            $receipt.source_commit = $rev[0].Trim()
        }
    }

    $env:DE441_PATH = $file.FullName
    foreach ($gate in @(
        @{
            suite = 'light_time_horizons'
            tests = @(
                'light_time_states_match_horizons_for_solar_system_targets',
                'real_kernel_covers_common_bodies_and_rejects_missing_emission_history'
            )
        },
        @{
            suite = 'aberrated_real_de441'
            tests = @(
                'real_de441_aberrated_pipeline_recovers_horizons_light_time_geometry',
                'real_de441_aberrated_batch_rates_follow_neighboring_full_pipeline_states'
            )
        }
    )) {
        $result = Invoke-CargoGate -Suite $gate.suite -ExpectedTests $gate.tests -Directory $reportDirectory
        $receipt.gates += $result
        Write-Host "$($gate.suite): $($result.status) (details: $($result.log))"
    }
    if (@($receipt.gates | Where-Object { $_.status -ne 'PASS' }).Count -gt 0) {
        throw 'At least one real-file scientific gate FAILED. Inspect individual logs; do not claim numerical validation.'
    }
    if ($receipt.gates.Count -ne 2) {
        throw 'Two scientific suites must actually execute; partial validation is insufficient.'
    }
    $receipt.status = 'PASS_LIMITED_LT_REFERENCE_AND_INTERNAL_RATES'
}
catch {
    $receipt.status = 'FAIL'
    $failure = $_.Exception.Message
    $receipt.error = $failure
}
finally {
    [Environment]::SetEnvironmentVariable('DE441_PATH', $priorDe441Path, 'Process')
    Pop-Location
    $receiptPath = Join-Path $reportDirectory 'receipt.json'
    [IO.File]::WriteAllText(
        $receiptPath,
        ($receipt | ConvertTo-Json -Depth 9),
        [Text.UTF8Encoding]::new($false)
    )
    Write-Host "Evidence saved locally to: $reportDirectory"
}
if ($null -ne $failure) {
    Write-Error "Real-DE441 validation failed: $failure"
    exit 1
}
Write-Host 'PASS: Horizons LT geometry and whole-pipeline real-file rate-consistency gates passed.'
Write-Host 'NOT established: independently validated annual-aberration boost, Horizons LT+S, date-frame corrections, Swiss parity.'
exit 0

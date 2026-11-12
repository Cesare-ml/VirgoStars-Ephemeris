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
    [string]$ExpectedSha256,
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

# Optional match against an out-of-band fingerprint supplied by caller.
# Merely recording SHA-256 is identity evidence, not authentication.
function Assert-ExpectedKernelHash {
    param([string]$ActualHash, [string]$ExpectedHash)
    if ([string]::IsNullOrWhiteSpace($ExpectedHash)) {
        return $false
    }
    if ($ExpectedHash -cnotmatch '^[0-9a-fA-F]{64}$') {
        throw '-ExpectedSha256 must have exactly 64 hexadecimal characters.'
    }
    if ($ActualHash -cnotmatch '^[0-9a-fA-F]{64}$') {
        throw 'Calculated DE441 SHA-256 is absent or malformed.'
    }
    if (-not [string]::Equals($ActualHash, $ExpectedHash, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'The local DE441 SHA-256 differs from the expected fingerprint.'
    }
    return $true
}

# Parse only the four explicitly referenced body names, without deriving
# science from a log with missing, duplicated or nonfinite measurements.
function Read-ResidualGroup {
    param([string]$Text, [string]$Pattern, [string[]]$Fields)
    [string[]]$expected = @('Moon', 'Venus', 'MarsBarycenter', 'JupiterBarycenter')
    $records = [regex]::Matches($Text, $Pattern)
    if ($records.Count -ne $expected.Count) {
        throw "Expected four numeric reference rows but found $($records.Count)."
    }
    $seen = @{}
    $rows = @()
    foreach ($match in $records) {
        $body = $match.Groups['target'].Value
        if ($seen.ContainsKey($body)) { throw "Repeated numeric row for $body." }
        $seen[$body] = $true
        $row = [ordered]@{ target = $body }
        foreach ($field in $Fields) {
            [double]$value = [double]::Parse(
                $match.Groups[$field].Value,
                [Globalization.NumberStyles]::Float,
                [Globalization.CultureInfo]::InvariantCulture
            )
            if (-not [double]::IsFinite($value) -or $value -lt 0.0) {
                throw "Invalid numeric residual: $body / $field."
            }
            $row[$field] = $value
        }
        $rows += [pscustomobject]$row
    }
    foreach ($body in $expected) {
        if (-not $seen.ContainsKey($body)) { throw "Missing $body numeric reference." }
    }
    $maximum = [ordered]@{}
    foreach ($field in $Fields) {
        [double]$highest = 0.0
        foreach ($row in $rows) {
            $highest = [math]::Max($highest, [double]$row.$field)
        }
        $maximum[$field] = $highest
    }
    return [pscustomobject]@{ measurements = $rows; maxima = $maximum }
}

function Read-ScientificMetrics {
    param([string]$Suite, [string[]]$Lines)
    $text = $Lines -join [Environment]::NewLine
    $n = '[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?'
    $prefix = '(?m)^(?<target>Moon|Venus|MarsBarycenter|JupiterBarycenter):[ \t]*'
    if ($Suite -eq 'light_time_horizons') {
        $pattern = $prefix +
            'position[ \t]+(?<position_max_component_km>' + $n + ')[ \t]+km,[ \t]+' +
            'Horizons velocity[ \t]+(?<horizons_velocity_max_component_km_per_s>' + $n + ')[ \t]+km/s,[ \t]+' +
            'finite-difference derivative[ \t]+(?<lt_derivative_max_component_km_per_s>' + $n + ')[ \t]+km/s,[ \t]+' +
            'delay[ \t]+(?<light_time_error_seconds>' + $n + ')[ \t]+s[ \t]*\r?$'
        return [ordered]@{
            horizons_lt_vectors = Read-ResidualGroup -Text $text -Pattern $pattern -Fields @(
                'position_max_component_km',
                'horizons_velocity_max_component_km_per_s',
                'lt_derivative_max_component_km_per_s',
                'light_time_error_seconds'
            )
        }
    }
    if ($Suite -eq 'aberrated_real_de441') {
        $vectorPattern = $prefix +
            'recovered Horizons LT vector error[ \t]+(?<recovered_lt_vector_max_component_km>' + $n + ')[ \t]+km,[ \t]+' +
            'LT range error[ \t]+(?<lt_range_error_km>' + $n + ')[ \t]+km,[ \t]+' +
            'delay error[ \t]+(?<light_time_error_seconds>' + $n + ')[ \t]+s[ \t]*\r?$'
        $ratePattern = $prefix +
            'max angular rate residual = [ \t]*(?<angular_rate_error_arcsec_per_tdb_day>' + $n + ')[ \t]+arcseconds/TDB-day;[ \t]+' +
            'radial rate residual = [ \t]*(?<radial_rate_error_au_per_tdb_day>' + $n + ')[ \t]+AU/TDB-day[ \t]*\r?$'
        return [ordered]@{
            recovered_lt_vectors = Read-ResidualGroup -Text $text -Pattern $vectorPattern -Fields @(
                'recovered_lt_vector_max_component_km', 'lt_range_error_km', 'light_time_error_seconds'
            )
            full_pipeline_rate_consistency = Read-ResidualGroup -Text $text -Pattern $ratePattern -Fields @(
                'angular_rate_error_arcsec_per_tdb_day', 'radial_rate_error_au_per_tdb_day'
            )
        }
    }
    throw "No metrics contract defined for suite '$Suite'."
}

# The thresholds below are TRANSCRIPTIONS of the current explicit Rust
# asserts in tests/light_time_horizons.rs and tests/aberrated_real_de441.rs.
# They do not establish independently justified scientific accuracy.
# Rust computes on full f64 precision; its assertions remain the authority.
# Decimal logs can round a value just below a strict bound up to equality.
# Accordingly, this *secondary* envelope only rejects PRINTED values > bound.
function Get-ReportedScientificLimits {
    param([string]$Suite, [System.Collections.IDictionary]$Metrics)

    $limits = [ordered]@{}
    if ($Suite -eq 'light_time_horizons') {
        $limits['horizons_lt_vectors'] = [ordered]@{
            position_max_component_km = 0.01
            horizons_velocity_max_component_km_per_s = 1e-6
            lt_derivative_max_component_km_per_s = 3e-6
            light_time_error_seconds = 1e-6
        }
    }
    elseif ($Suite -eq 'aberrated_real_de441') {
        $limits['recovered_lt_vectors'] = [ordered]@{
            recovered_lt_vector_max_component_km = 0.01
            lt_range_error_km = 0.01
            light_time_error_seconds = 1e-6
        }
        # Rust checks angle_error < 2e-4 deg/TDB day. Report uses
        # arcseconds/TDB day: 2e-4 * 3600 = 0.72.
        $limits['full_pipeline_rate_consistency'] = [ordered]@{
            angular_rate_error_arcsec_per_tdb_day = 0.72
            radial_rate_error_au_per_tdb_day = 1e-7
        }
    }
    else {
        throw "No pinned scientific thresholds for suite $Suite."
    }

    $results = [ordered]@{}
    foreach ($group in $limits.Keys) {
        if (-not $Metrics.Contains($group)) {
            throw "Required scientific metric group $group is missing."
        }
        $maxima = $Metrics[$group].maxima
        $ratios = [ordered]@{}
        foreach ($field in $limits[$group].Keys) {
            if (-not $maxima.Contains($field)) {
                throw "Missing maximum for scientific metric $group.$field."
            }
            [double]$observed = $maxima[$field]
            [double]$upper = $limits[$group][$field]
            if (-not [double]::IsFinite($observed) -or $observed -lt 0.0 -or
                -not [double]::IsFinite($upper) -or $upper -le 0.0) {
                throw "Invalid finite scientific limit check for $group.$field."
            }
            if ($observed -gt $upper) {
                throw "Reported residual $group.$field = $observed exceeds the Rust-test threshold $upper."
            }
            $ratios[$field] = $observed / $upper
        }
        $results[$group] = [ordered]@{
            source = $(if ($Suite -eq 'light_time_horizons') {
                'crates/ephemeris-core/tests/light_time_horizons.rs'
            } else {
                'crates/ephemeris-core/tests/aberrated_real_de441.rs'
            })
            declared_strict_upper_limits = $limits[$group]
            reported_maximum_fractions_of_limit = $ratios
            printed_values_at_or_below_limit = $true
            note = 'Secondary check on rounded logs only. The original Rust f64 strict inequalities decide PASS.'
        }
    }
    return $results
}

function Invoke-CargoGate {
    param(
        [string]$Suite,
        [string[]]$ExpectedTests,
        [string]$Directory
    )
    [string[]]$cargoArguments = @(
        'test', '--workspace', '--locked',
        '--test', $Suite, '--', '--ignored', '--nocapture'
    )
    [string[]]$lines = @(& cargo @cargoArguments 2>&1 | ForEach-Object { "$_" })
    [int]$code = $LASTEXITCODE
    $logName = ($Suite + '.log')
    [IO.File]::WriteAllLines(
        (Join-Path $Directory $logName),
        $lines,
        [Text.UTF8Encoding]::new($false)
    )
    $pass = Test-GatePass -Lines $lines -ExitCode $code -ExpectedTests $ExpectedTests
    $metrics = $null
    $metricError = $null
    if ($pass -and $Suite -in @('light_time_horizons', 'aberrated_real_de441')) {
        try {
            $metrics = Read-ScientificMetrics -Suite $Suite -Lines $lines
        }
        catch {
            $metricError = $_.Exception.Message
            $pass = $false
        }
    }
    return [pscustomobject]@{
        suite = $Suite
        command = ('cargo ' + ($cargoArguments -join ' '))
        expected_tests = $ExpectedTests
        exit_code = $code
        status = $(if ($pass) { 'PASS' } else { 'FAIL' })
        log = $logName
        metrics = $metrics
        metric_parse_error = $metricError
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
        # Actually invoke the same Cargo wrapper with a deliberately fake
        # native cargo.cmd. The Windows-only fake verifies every argument,
        # including --ignored, before emitting two simulated PASS lines.
        # This catches a missing / wrong splatted argument variable.
        $mockCargo = Join-Path $temp 'cargo.cmd'
        [string[]]$mockLines = @(
            '@echo off',
            'if not "%1"=="test" exit /b 91',
            'if not "%2"=="--workspace" exit /b 92',
            'if not "%3"=="--locked" exit /b 93',
            'if not "%4"=="--test" exit /b 94',
            'if not "%5"=="fake_suite" exit /b 95',
            'if not "%6"=="--" exit /b 96',
            'if not "%7"=="--ignored" exit /b 97',
            'if not "%8"=="--nocapture" exit /b 98',
            'if not "%9"=="" exit /b 99',
            'echo running 2 tests',
            'echo test real_first ... ok',
            'echo test real_second ... ok',
            'echo test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured',
            'exit /b 0'
        )
        [IO.File]::WriteAllLines($mockCargo, $mockLines, [Text.Encoding]::ASCII)
        [string]$originalPath = $env:PATH
        try {
            $env:PATH = $temp + [IO.Path]::PathSeparator + $originalPath
            $mock = Invoke-CargoGate -Suite 'fake_suite' -ExpectedTests $tests -Directory $temp
            if ($mock.status -ne 'PASS' -or $mock.exit_code -ne 0) {
                throw 'Cargo invocation did not pass the complete explicit argument list.'
            }
            if (-not (Test-Path -LiteralPath (Join-Path $temp 'fake_suite.log'))) {
                throw 'Mock Cargo invocation did not preserve its test transcript.'
            }
        }
        finally {
            $env:PATH = $originalPath
        }
        Write-Host '[PASS] Real Cargo wrapper passes exact --ignored arguments to a mock executable.'

        $knownHash = 'a' * 64
        if (Assert-ExpectedKernelHash -ActualHash $knownHash -ExpectedHash '') {
            throw 'No expected SHA-256 must remain unverified.'
        }
        if (-not (Assert-ExpectedKernelHash -ActualHash $knownHash -ExpectedHash ('A' * 64))) {
            throw 'Case-insensitive valid SHA-256 match was rejected.'
        }
        # Parenthesize each string repetition: PowerShell otherwise binds
        # the comma expressions as an Object[] multiplier to the first '*'.
        foreach ($bad in @(('a' * 63), ('z' * 64), ('b' * 64))) {
            $rejected = $false
            try {
                $null = Assert-ExpectedKernelHash -ActualHash $knownHash -ExpectedHash $bad
            }
            catch { $rejected = $true }
            if (-not $rejected) { throw 'Malformed or mismatched SHA-256 was accepted.' }
        }
        Write-Host '[PASS] Expected SHA-256 gate rejects mismatch/malformed and records no expectation.'
        # These are invented METRIC LOG LINES, not real observations.
        # Validate parsing and fail-closed behavior on partial duplicates.
        $metricBodies = @('Moon', 'Venus', 'MarsBarycenter', 'JupiterBarycenter')
        [string[]]$ltLines = @($metricBodies | ForEach-Object {
            "$($_): position 0.000000005 km, Horizons velocity 0.000000000001 km/s, finite-difference derivative 0.000000000002 km/s, delay 0.000000000001 s"
        })
        [string[]]$apparentLines = @($metricBodies | ForEach-Object {
            "$($_): recovered Horizons LT vector error 0.000000006 km,              LT range error 0.000000007 km, delay error 0.000000000003 s"
            "$($_): max angular rate residual = 0.000000008 arcseconds/TDB-day;              radial rate residual = 1.0000000000e-10 AU/TDB-day"
        })
        $ltMetrics = Read-ScientificMetrics -Suite 'light_time_horizons' -Lines $ltLines
        $apparentMetrics = Read-ScientificMetrics -Suite 'aberrated_real_de441' -Lines $apparentLines
        if ($ltMetrics.horizons_lt_vectors.measurements.Count -ne 4 -or
            $apparentMetrics.recovered_lt_vectors.measurements.Count -ne 4 -or
            $apparentMetrics.full_pipeline_rate_consistency.measurements.Count -ne 4) {
            throw 'Expected all four reference bodies in all three numeric groups.'
        }
        if ($ltMetrics.horizons_lt_vectors.maxima.position_max_component_km -ne 0.000000005 -or
            $apparentMetrics.recovered_lt_vectors.maxima.lt_range_error_km -ne 0.000000007 -or
            $apparentMetrics.full_pipeline_rate_consistency.maxima.radial_rate_error_au_per_tdb_day -ne 1e-10) {
            throw 'The metric maxima or scientific unit fields were parsed incorrectly.'
        }
        foreach ($corrupt in @(
            $ltLines[0..2],
            ($ltLines[0..2] + $ltLines[0]),
            @($ltLines | ForEach-Object { $_.Replace('0.000000005', 'NaN') }),
            @($ltLines | ForEach-Object { $_.Replace('0.000000005', '-0.000000005') }),
            @($ltLines | ForEach-Object { $_.Replace('0.000000005', '1e999') })
        )) {
            $rejected = $false
            try {
                $null = Read-ScientificMetrics -Suite 'light_time_horizons' -Lines $corrupt
            }
            catch { $rejected = $true }
            if (-not $rejected) {
                throw 'Incomplete, duplicated, NaN, negative or overflowed metrics were accepted.'
            }
        }
        foreach ($corrupt in @(
            $apparentLines[0..6],
            @($apparentLines | ForEach-Object {
                $_.Replace('1.0000000000e-10', 'NaN')
            })
        )) {
            $rejected = $false
            try {
                $null = Read-ScientificMetrics -Suite 'aberrated_real_de441' -Lines $corrupt
            }
            catch { $rejected = $true }
            if (-not $rejected) {
                throw 'Missing or malformed apparent/rate metric group was accepted.'
            }
        }
        Write-Host '[PASS] Numeric Horizons LT and aberrated-rate receipts require four finite body records each.'
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
    schema_version = 3
    status = 'NOT_RUN'
    run_timestamp_utc = $timestamp
    tool = 'VirgoStars-Ephemeris real classic-DE441 receipt v1'
    repository = 'https://github.com/Cesare-ml/VirgoStars-Ephemeris'
    source_commit = $null
    source_tree_clean = $false
    file = [ordered]@{
        filename = $null
        bytes = $null
        sha256 = $null
        expected_sha256 = $null
        sha256_matched_expected = $false
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
    if (-not [string]::IsNullOrWhiteSpace($ExpectedSha256)) {
        $receipt.file.expected_sha256 = $ExpectedSha256.ToLowerInvariant()
    }
    $receipt.file.sha256_matched_expected = Assert-ExpectedKernelHash -ActualHash $receipt.file.sha256 -ExpectedHash $ExpectedSha256
    $receipt.rustc_version = ((& rustc --version) -join '')
    $receipt.cargo_version = ((& cargo --version) -join '')
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        throw 'Git is unavailable; a reproducible source commit is required.'
    }
    $rev = @(& git rev-parse --verify HEAD 2>$null)
    if ($LASTEXITCODE -ne 0 -or $rev.Count -ne 1 -or $rev[0] -notmatch '^[0-9a-fA-F]{40}$') {
        throw 'Cannot identify the Git source commit for scientific reproducibility.'
    }
    $receipt.source_commit = $rev[0].Trim()
    $dirty = @(& git status --porcelain --untracked-files=normal 2>$null)
    if ($LASTEXITCODE -ne 0) {
        throw 'Cannot inspect the Git worktree for uncommitted changes.'
    }
    if ($dirty.Count -ne 0) {
        throw 'Git worktree is dirty. Commit or remove local code changes before scientific validation.'
    }
    $receipt.source_tree_clean = $true

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

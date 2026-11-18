# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
# The Original Developer is the Initial Developer.
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
# Contributors retain applicable rights in their own contributions under the terms of this License.
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0
# Modification (2026-11-18): non-destructive Windows offline mobile-pack pipeline.

# Example, from the repository root (PowerShell 5.1 or 7):
# powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_de441_mobile_windows.ps1 -OutputDirectory "D:\VirgoStars-DE441" -DownloadIfMissing
# Alternative if a classic DE441 file was obtained already:
# powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_de441_mobile_windows.ps1 -SourcePath "D:\Data\de441.eph" -OutputDirectory "D:\VirgoStars-DE441"

[CmdletBinding()]
param(
    [string] $SourcePath,
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string] $OutputDirectory,
    [switch] $DownloadIfMissing,
    [switch] $BaseOnly,
    [string] $ExpectedSourceSha256
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Require-Command([string] $Name) {
    if (-not (Get-Command -Name $Name -ErrorAction SilentlyContinue)) {
        throw "Comando non trovato: $Name. Installa Python 3.10+ e Rust (rustup con rustfmt/clippy), riapri PowerShell, poi ripeti."
    }
}

function Invoke-Checked([string] $Name, [string[]] $Arguments) {
    Write-Host ">> $Name $($Arguments -join ' ')" -ForegroundColor Cyan
    & $Name @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Comando fallito con exit code $LASTEXITCODE : $Name"
    }
}

$separators = [char[]]@('\', '/')
$repository = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd($separators)
$output = [System.IO.Path]::GetFullPath($OutputDirectory).TrimEnd($separators)
$comparison = [System.StringComparison]::OrdinalIgnoreCase
if ($output.Equals([System.IO.Path]::GetPathRoot($output).TrimEnd($separators), $comparison)) {
    throw "La cartella di output deve essere una sottocartella dedicata, non la radice del disco."
}
if ($output.Equals($repository, $comparison) -or
    $output.StartsWith($repository + [System.IO.Path]::DirectorySeparatorChar, $comparison)) {
    throw "La cartella di output deve stare FUORI dal repository: nessun file JPL dentro Git."
}

if ([string]::IsNullOrWhiteSpace($SourcePath)) {
    if (-not $DownloadIfMissing) {
        throw "Specifica -SourcePath oppure -DownloadIfMissing per il file JPL DE441 classic."
    }
    $source = Join-Path $output 'source\linux_m13000p17000.441'
} else {
    $source = [System.IO.Path]::GetFullPath($SourcePath)
}
if ($source.Equals($repository, $comparison) -or
    $source.StartsWith($repository + [System.IO.Path]::DirectorySeparatorChar, $comparison)) {
    throw "Anche il file DE441 originale deve essere conservato FUORI dal repository Git."
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedSourceSha256) -and
    $ExpectedSourceSha256 -notmatch '^[0-9a-fA-F]{64}$') {
    throw "-ExpectedSourceSha256 deve avere esattamente 64 caratteri esadecimali."
}

Require-Command 'python'
Require-Command 'cargo'
Invoke-Checked 'python' @('-c', 'import sys; assert sys.version_info >= (3, 10), "Python 3.10+ required"')
New-Item -ItemType Directory -Path $output -Force | Out-Null
$packDirectory = Join-Path $output 'packs'
$transcript = Join-Path $output ("de441-run-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + ".log")

$transcriptStarted = $false
$originalPythonSetting = [Environment]::GetEnvironmentVariable('PYTHONDONTWRITEBYTECODE', 'Process')
try {
    Start-Transcript -LiteralPath $transcript -NoClobber | Out-Null
    $transcriptStarted = $true
    [Environment]::SetEnvironmentVariable('PYTHONDONTWRITEBYTECODE', '1', 'Process')

    if (-not (Test-Path -LiteralPath $source -PathType Leaf) -and -not $DownloadIfMissing) {
        throw "File DE441 mancante: $source"
    }
    if ([System.IO.Path]::GetFullPath($source).Equals(
        [System.IO.Path]::GetFullPath((Join-Path $packDirectory 'de441-core.eph')),
        $comparison)) {
        throw "Sorgente e file output non possono coincidere."
    }

    Push-Location $repository
    try {
        Write-Host "FASE 1/4: formattazione, suite Rust e Clippy" -ForegroundColor Green
        Invoke-Checked 'cargo' @('fmt', '--all', '--', '--check')
        Invoke-Checked 'cargo' @('test', '--workspace')
        Invoke-Checked 'cargo' @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings')

        Write-Host "FASE 2/4: regressioni Python e controllo dati esterni" -ForegroundColor Green
        Invoke-Checked 'python' @('-m', 'unittest', 'discover', '-s', 'scripts/tests', '-p', 'test_*.py', '-q')
        Invoke-Checked 'python' @('scripts/check_repository_data_hygiene.py')

        Write-Host "FASE 3/4: estrazione DE441, checksum e parity Rust" -ForegroundColor Green
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            $sourceDirectory = Split-Path -Parent $source
            New-Item -ItemType Directory -Path $sourceDirectory -Force | Out-Null
            $partial = "$source.download-partial"
            if (Test-Path -LiteralPath $partial) {
                throw "Download incompleto gia' presente: $partial. Ispezionalo prima di riprovare."
            }
            $rootDrive = [System.IO.Path]::GetPathRoot($source)
            if ($rootDrive -and (Test-Path -LiteralPath $rootDrive)) {
                $drive = [System.IO.DriveInfo]::new($rootDrive)
                if ($drive.IsReady -and $drive.AvailableFreeSpace -lt 4GB) {
                    throw "Meno di 4 GB liberi sul disco per scaricare il DE441 originale."
                }
            }
            $url = 'https://ssd.jpl.nasa.gov/ftp/eph/planets/Linux/de441/linux_m13000p17000.441'
            Write-Host "Scarico circa 2.6 GB dall'archivio ufficiale JPL: $url" -ForegroundColor Yellow
            Invoke-WebRequest -Uri $url -OutFile $partial -UseBasicParsing
            if (-not (Test-Path -LiteralPath $partial -PathType Leaf)) {
                throw "Download non completato."
            }
            # The builder independently verifies DE441 magic, coverage and SHA-256.
            Move-Item -LiteralPath $partial -Destination $source -ErrorAction Stop
        }

        $selection = if ($BaseOnly) { 'core' } else { 'all' }
        $buildArgs = @(
            'scripts/build_de441_mobile_packs.py', 'build',
            '--source', $source,
            '--output-dir', $packDirectory,
            '--packs', $selection
        )
        if (-not [string]::IsNullOrWhiteSpace($ExpectedSourceSha256)) {
            $buildArgs += @('--expected-source-sha256', $ExpectedSourceSha256)
        }
        Invoke-Checked 'python' $buildArgs

        Write-Host "FASE 4/4: verifica finale e report leggibile" -ForegroundColor Green
        $manifest = Join-Path $packDirectory 'de441-mobile-manifest-v1.json'
        Invoke-Checked 'python' @('scripts/build_de441_mobile_packs.py', 'verify', '--manifest', $manifest)
        $result = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
        $report = Join-Path $packDirectory 'de441-mobile-report.txt'
        $rows = New-Object System.Collections.Generic.List[string]
        $rows.Add('VirgoStars - DE441 mobile offline pack report')
        $rows.Add(('Generated UTC: ' + $result.created_at_utc))
        $rows.Add(('Source SHA-256: ' + $result.source.sha256))
        $rows.Add(('Source bytes: ' + $result.source.bytes))
        $rows.Add(('Distribution rights: ' + $result.distribution_rights))
        $rows.Add(('Independent JPL validation: NOT PERFORMED'))
        $rows.Add('')
        [long] $totalBytes = 0
        foreach ($item in $result.packages) {
            if ($item.status -ne 'built') {
                $rows.Add(("{0}: NOT BUILT" -f $item.id))
                continue
            }
            $artifact = $item.artifact
            $mb = [Math]::Round(([double] $artifact.bytes / 1000000.0), 2)
            $mib = [Math]::Round(([double] $artifact.bytes / 1048576.0), 2)
            $totalBytes += [long] $artifact.bytes
            $rows.Add(("{0}: {1} bytes ({2} MB / {3} MiB) | {4}-{5}" -f
                $item.id, $artifact.bytes, $mb, $mib,
                $item.first_year, $item.last_year_inclusive))
            $rows.Add(('  SHA-256: ' + $artifact.sha256))
            $rows.Add(('  Parity epochs: ' + $artifact.parity.compared_epochs))
            $rows.Add(('  Parity state pairs: ' + $artifact.parity.compared_state_pairs))
        }
        $rows.Add('')
        $rows.Add(('Total generated bytes: ' + $totalBytes))
        $rows.Add('These checks verify source/slice identity only, not JPL scientific accuracy.')
        $reportStream = [System.IO.File]::Open(
            $report, [System.IO.FileMode]::CreateNew,
            [System.IO.FileAccess]::Write, [System.IO.FileShare]::None
        )
        $writer = [System.IO.StreamWriter]::new(
            $reportStream, [System.Text.UTF8Encoding]::new($false)
        )
        try {
            foreach ($row in $rows) { $writer.WriteLine($row) }
            $writer.Flush()
        } finally {
            $writer.Dispose()
        }
        Write-Host "COMPLETATO. Report: $report" -ForegroundColor Green
        Write-Host "Manifest: $manifest" -ForegroundColor Green
        Write-Host "Log: $transcript" -ForegroundColor Green
        Write-Host "I dati NON sono approvati per ridistribuzione pubblica." -ForegroundColor Yellow
    } finally {
        Pop-Location
    }
} finally {
    [Environment]::SetEnvironmentVariable(
        'PYTHONDONTWRITEBYTECODE', $originalPythonSetting, 'Process'
    )
    if ($transcriptStarted) { Stop-Transcript | Out-Null }
}

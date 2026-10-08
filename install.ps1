#Requires -Version 5.1
<#
.SYNOPSIS
  Hyper Engine 'hfe' installer for Windows (PowerShell 5.1+).

.DESCRIPTION
  Downloads the prebuilt hfe.exe from the official GitHub release, verifies its
  SHA-256 checksum, installs it, and (optionally) adds it to your user PATH.
  Installs only the 'hfe' CLI; the Tauri GUI launcher is a separate download.

.EXAMPLE
  irm https://raw.githubusercontent.com/Xznder1984/Hyper-FNF-Engine/main/install.ps1 | iex

.EXAMPLE
  .\install.ps1 -Version 1.0.4 -Dir "$env:USERPROFILE\bin" -NoModifyPath

.NOTES
  Environment overrides: HFE_VERSION, HFE_DIR, HFE_REPO, GITHUB_TOKEN (optional).
#>
[CmdletBinding()]
param(
    [string]$Version = $(if ($env:HFE_VERSION) { $env:HFE_VERSION } else { 'latest' }),
    [string]$Dir     = $(if ($env:HFE_DIR)     { $env:HFE_DIR }     else { '' }),
    [string]$Repo    = $(if ($env:HFE_REPO)    { $env:HFE_REPO }    else { 'Xznder1984/Hyper-FNF-Engine' }),
    [switch]$NoModifyPath,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}

function Info($m) { Write-Host "  $m" }
function Fail($m) { Write-Host "error: $m" -ForegroundColor Red; exit 1 }

# --- platform detection ----------------------------------------------------
$raw = $env:PROCESSOR_ARCHITECTURE
if (-not $raw) { $raw = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() }
switch ($raw.ToUpperInvariant()) {
    'AMD64' { $arch = 'x86_64' }
    'ARM64' { $arch = 'aarch64' }
    default { Fail "unsupported architecture: $raw" }
}
$target = "$arch-pc-windows-msvc"

if (-not $Dir) { $Dir = Join-Path $env:LOCALAPPDATA 'hyper-engine\bin' }

# --- resolve version -------------------------------------------------------
if ($Version -eq 'latest') {
    Info "resolving latest Hyper Engine release from $Repo ..."
    $headers = @{ 'User-Agent' = 'hyper-engine-installer'; 'Accept' = 'application/vnd.github+json' }
    if ($env:GITHUB_TOKEN) { $headers['Authorization'] = "Bearer $($env:GITHUB_TOKEN)" }
    try {
        $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $headers
        $tag = $rel.tag_name
    } catch {
        Fail "no published release found for $Repo ($($_.Exception.Message))"
    }
} else {
    $tag = $Version
}
if (-not $tag) { Fail 'could not determine a release tag' }

# --- confirm ---------------------------------------------------------------
$interactive = (-not [Console]::IsInputRedirected) -and [Environment]::UserInteractive
if (-not $Yes -and $interactive) {
    Write-Host "Install hfe $tag ($target) to $Dir? [y/N] " -NoNewline
    $reply = Read-Host
    if ($reply -notmatch '^(y|yes)$') { Write-Host 'aborted'; exit 0 }
}

$asset = "hfe-$target.zip"
$base  = "https://github.com/$Repo/releases/download/$tag"
$url   = "$base/$asset"

New-Item -ItemType Directory -Force -Path $Dir | Out-Null
$tmp = Join-Path $env:TEMP ("hfe-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

try {
    $zip = Join-Path $tmp $asset
    Info "downloading $asset ($tag)"
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    } catch {
        Fail "download failed: $url ($($_.Exception.Message))"
    }

    # --- checksum ----------------------------------------------------------
    $sha = $null
    try {
        $resp = Invoke-WebRequest -Uri "$url.sha256" -UseBasicParsing
        $sha = $resp.Content
    } catch { $sha = $null }
    if ($sha) {
        if ($sha -is [byte[]]) { $sha = [Text.Encoding]::UTF8.GetString($sha) }
        $want = ($sha.Trim() -split '\s+')[0].ToLowerInvariant()
        $have = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
        if ($have -ne $want) { Fail "checksum mismatch (have $have, want $want)" }
        Info 'sha256 verified'
    } else {
        Info 'warning: no .sha256 published for this release; skipping verification'
    }

    # --- extract + install -------------------------------------------------
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exe = Get-ChildItem -Path $tmp -Recurse -Filter 'hfe.exe' | Select-Object -First 1
    if (-not $exe) { Fail 'archive did not contain hfe.exe' }
    $out = Join-Path $Dir 'hfe.exe'
    Copy-Item -Path $exe.FullName -Destination $out -Force

    # --- PATH --------------------------------------------------------------
    if (-not $NoModifyPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @()
        if ($userPath) { $parts = $userPath.Split(';') | Where-Object { $_ } }
        if ($parts -notcontains $Dir) {
            [Environment]::SetEnvironmentVariable('Path', (($parts + $Dir) -join ';'), 'User')
            Info "added $Dir to your user PATH (restart your shell)"
        }
        $env:Path = "$env:Path;$Dir"
    } else {
        Info "PATH unchanged; add $Dir manually if needed"
    }

    Write-Host "installed hfe $tag -> $out"
    Write-Host 'run: hfe --help'
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
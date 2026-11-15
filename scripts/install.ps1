<#
.SYNOPSIS
  Dial Control installer for Windows 10/11 (per-user, no administrator rights needed).

.EXAMPLE
  irm https://github.com/Yuvraj675/dial-control/releases/latest/download/install.ps1 | iex

.EXAMPLE
  # Specific version, or review before running:
  $env:DIAL_CONTROL_VERSION = "1.0.0"; irm https://github.com/Yuvraj675/dial-control/releases/latest/download/install.ps1 | iex
  irm https://github.com/Yuvraj675/dial-control/releases/latest/download/install.ps1 -OutFile install.ps1; notepad install.ps1; .\install.ps1

Downloads the setup .exe and SHA256SUMS from the GitHub release, verifies the checksum, then runs the
installer silently. Environment: DIAL_CONTROL_VERSION, DIAL_CONTROL_BASE_URL (testing).
#>
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = 'Yuvraj675/dial-control'
$Base = if ($env:DIAL_CONTROL_BASE_URL) { $env:DIAL_CONTROL_BASE_URL } else { "https://github.com/$Repo/releases" }
$Version = ($env:DIAL_CONTROL_VERSION -replace '^v', '')

function Fail($msg) { Write-Host " error $msg" -ForegroundColor Red; throw $msg }

if (-not [Environment]::Is64BitOperatingSystem) { Fail 'Only 64-bit Windows builds are published.' }
if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { Write-Host ' warn ARM64 detected; the x64 build runs under emulation.' -ForegroundColor Yellow }

if (-not $Version) {
  Write-Host '==> Looking up the latest release'
  try {
    $api = "https://api.github.com/repos/$Repo/releases/latest"
    $Version = ((Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent' = 'dial-control-installer' }).tag_name) -replace '^v', ''
  } catch { Fail 'No published release found (has one been released yet?).' }
}
Write-Host "==> Dial Control v$Version on Windows"

$asset = "Dial-Control_${Version}_x64-setup.exe"
$url = "$Base/download/v$Version"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("dial-control-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Host "==> Downloading $asset"
  Invoke-WebRequest -Uri "$url/$asset" -OutFile (Join-Path $tmp $asset) -UseBasicParsing
  Invoke-WebRequest -Uri "$url/SHA256SUMS" -OutFile (Join-Path $tmp 'SHA256SUMS') -UseBasicParsing

  $line = Get-Content (Join-Path $tmp 'SHA256SUMS') | Where-Object { $_ -match "\s\*?$([regex]::Escape($asset))$" } | Select-Object -First 1
  if (-not $line) { Fail "$asset is not listed in SHA256SUMS" }
  $want = ($line -split '\s+')[0].ToLower()
  $got = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $tmp $asset)).Hash.ToLower()
  if ($want -ne $got) { Fail "Checksum mismatch (expected $want, got $got); not installing." }
  Write-Host ' ok checksum verified' -ForegroundColor Green

  Write-Host '==> Installing (per-user, silent)'
  $p = Start-Process -FilePath (Join-Path $tmp $asset) -ArgumentList '/S' -Wait -PassThru
  if ($p.ExitCode -ne 0) { Fail "Installer exited with code $($p.ExitCode)" }
}
finally { Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue }

Write-Host ''
Write-Host " ok Dial Control v$Version installed." -ForegroundColor Green
Write-Host @'

Next steps
  * Launch "Dial Control" from the Start menu (it lives in the system tray).
  * In your earbuds' companion app set: double press = Next track, triple press = Previous track.
  * No earbuds? Press Win+Shift+D to open the wheel from the keyboard.
  * If SmartScreen warns, the installer is unsigned: choose "More info" > "Run anyway".
'@

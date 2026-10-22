# Dial Control - Windows Local Build Script
# Run this inside PowerShell on your Windows VM or laptop:

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Building Dial Control for Windows (x64)                 " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Check prerequisites
if (-not (Get-Command "pnpm" -ErrorAction SilentlyContinue)) {
    Write-Warning "pnpm is not found. Attempting to run with npm..."
    $PKG_MGR = "npm"
} else {
    $PKG_MGR = "pnpm"
}

# 2. Install frontend dependencies
Write-Host "`n==> [1/2] Installing frontend dependencies..." -ForegroundColor Yellow
& $PKG_MGR install

# 3. Compile Windows executable and installers
Write-Host "`n==> [2/2] Building release binary and installers (.exe & .msi)..." -ForegroundColor Yellow
npx @tauri-apps/cli build

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "  Build Successful! Output packages:" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Get-ChildItem -Path "src-tauri/target/release/bundle" -Recurse -Include *.msi,*.exe | Select-Object -ExpandProperty FullName

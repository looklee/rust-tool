<#
.SYNOPSIS
    Unified AI Coding Assistant - One-click run script for Windows
#>

param(
    [string[]]$Args
)

function Write-Banner {
    Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Blue
    Write-Host "║              Unified AI Coding Assistant                  ║" -ForegroundColor Blue
    Write-Host "║                  All-in-One Solution                      ║" -ForegroundColor Blue
    Write-Host "╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Blue
}

function Test-RustInstalled {
    Write-Host "Checking Rust installation..." -ForegroundColor Cyan
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    if (-not $rustc) {
        Write-Host "Rust not found. Installing Rust..." -ForegroundColor Yellow
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
        Start-Process -Wait -FilePath "$env:TEMP\rustup-init.exe" -ArgumentList "-y"
        $env:PATH += ";$env:USERPROFILE\.cargo\bin"
        Write-Host "Rust installed successfully!" -ForegroundColor Green
    } else {
        Write-Host "Rust is already installed." -ForegroundColor Green
    }
}

function Build-Project {
    Write-Host "Building project..." -ForegroundColor Cyan
    
    if (Test-Path "target\release\code.exe") {
        Write-Host "Found existing binary. Rebuilding..." -ForegroundColor Yellow
    }
    
    cargo build --release -p common
    cargo build --release -p code
    
    if (Test-Path "target\release\code.exe") {
        Write-Host "Build successful!" -ForegroundColor Green
    } else {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
}

function Run-Code {
    Write-Host "Starting AI Coding Assistant..." -ForegroundColor Cyan
    Write-Host ""
    
    $binary = ".\target\release\code.exe"
    
    if ($Args.Length -eq 0) {
        & $binary
    } else {
        & $binary $Args
    }
}

Write-Banner
Write-Host ""

Test-RustInstalled
Write-Host ""

Build-Project
Write-Host ""

Run-Code
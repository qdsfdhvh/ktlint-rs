<#
.SYNOPSIS
  Install ktlint-rs on Windows from a GitHub release.

.DESCRIPTION
  Downloads the prebuilt ktlint-rs.exe from GitHub release and adds
  it to the current user's PATH.

.PARAMETER Version
  Release tag to install (default: latest).

.PARAMETER Repo
  Repository slug owner/name (default: qdsfdhvh/ktlint-rs).

.PARAMETER InstallDir
  Install directory (default: $env:USERPROFILE\.ktlint-rs\bin).

.EXAMPLE
  iwr -useb https://github.com/qdsfdhvh/ktlint-rs/releases/latest/download/install.ps1 | iex
#>
param(
  [string]$Version = "latest",
  [string]$Repo = "qdsfdhvh/ktlint-rs",
  [string]$InstallDir = "$env:USERPROFILE\.ktlint-rs\bin"
)

$ErrorActionPreference = "Stop"

if ($Version -eq "latest") {
  $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
  $Version = $release.tag_name
}

$url = "https://github.com/$Repo/releases/download/$Version/ktlint-rs-x86_64-pc-windows-msvc.exe"
Write-Host ":: Downloading ktlint-rs $Version..." -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Invoke-WebRequest -Uri $url -OutFile "$InstallDir\ktlint-rs.exe"

# Add to PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
  [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
  Write-Host "! Added $InstallDir to your PATH. Restart your terminal." -ForegroundColor Yellow
}

Write-Host ":: ktlint-rs installed to $InstallDir\ktlint-rs.exe" -ForegroundColor Cyan

# PowerShell Script to Configure Docker to Use D: Drive
# Run as Administrator

Write-Host "🐕 CYNIC Docker Configuration" -ForegroundColor Cyan
Write-Host "Redirecting Docker to D: drive..." -ForegroundColor Yellow
Write-Host ""

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "ERROR: This script must run as Administrator" -ForegroundColor Red
    Write-Host "Right-click PowerShell -> Run as Administrator" -ForegroundColor Yellow
    exit 1
}

# Step 1: Stop Docker Desktop
Write-Host "⏹️  Stopping Docker Desktop..." -ForegroundColor Yellow
taskkill /IM "Docker Desktop.exe" /F 2>$null | Out-Null
taskkill /IM "com.docker.service.exe" /F 2>$null | Out-Null
Start-Sleep -Seconds 3

# Step 2: Create D:\DockerData directory
Write-Host "📁 Creating D:\DockerData..." -ForegroundColor Yellow
$dockerDataPath = "D:\DockerData"
if (-not (Test-Path $dockerDataPath)) {
    New-Item -ItemType Directory -Path $dockerDataPath -Force | Out-Null
    Write-Host "✅ Created $dockerDataPath" -ForegroundColor Green
} else {
    Write-Host "✅ $dockerDataPath already exists" -ForegroundColor Green
}

# Step 3: Set permissions
Write-Host "🔐 Setting permissions..." -ForegroundColor Yellow
$username = $env:USERNAME
icacls $dockerDataPath /grant:r "$username`:F" /t /c | Out-Null
Write-Host "✅ Permissions set" -ForegroundColor Green

# Step 4: Create daemon.json
Write-Host "⚙️  Creating Docker daemon.json..." -ForegroundColor Yellow
$dockerConfigPath = "$env:APPDATA\Docker"
$daemonJsonPath = Join-Path $dockerConfigPath "daemon.json"

# Ensure Docker config directory exists
if (-not (Test-Path $dockerConfigPath)) {
    New-Item -ItemType Directory -Path $dockerConfigPath -Force | Out-Null
}

$daemonConfig = @{
    "data-root" = "D:\DockerData"
    "registry-mirrors" = @()
    "insecure-registries" = @()
} | ConvertTo-Json

Set-Content -Path $daemonJsonPath -Value $daemonConfig -Encoding UTF8
Write-Host "✅ Created $daemonJsonPath" -ForegroundColor Green

# Step 5: Restart Docker Desktop
Write-Host "🚀 Starting Docker Desktop..." -ForegroundColor Yellow
& "C:\Program Files\Docker\Docker\Docker Desktop.exe" | Out-Null

# Wait for Docker to start
Write-Host "⏳ Waiting for Docker to initialize (30 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 30

# Step 6: Verify configuration
Write-Host "🔍 Verifying configuration..." -ForegroundColor Yellow
$dockerInfo = docker info 2>$null
if ($dockerInfo -match "D:\\DockerData") {
    Write-Host "✅ Docker Root Dir is now: D:\DockerData" -ForegroundColor Green
} else {
    Write-Host "⚠️  Could not verify configuration" -ForegroundColor Yellow
    Write-Host "   Run manually: docker info | findstr 'Root Dir'" -ForegroundColor Yellow
}

# Final message
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✅ Docker configured to use D: drive" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next step:" -ForegroundColor Yellow
Write-Host "  cd C:\Users\$username\Desktop\asdfasdfa\CYNIC" -ForegroundColor White
Write-Host "  start_organism.cmd" -ForegroundColor White
Write-Host ""
Write-Host "🐕 CYNIC will now awaken with D: drive space." -ForegroundColor Cyan
Write-Host ""

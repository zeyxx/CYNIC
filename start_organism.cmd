@echo off
REM 🐕 CYNIC Organism Launcher (Windows)
REM Starts: Dashboard + Organism + Infrastructure

setlocal enabledelayedexpansion

cd /d "%~dp0\cynic" || exit /b 1

echo.
echo ═══════════════════════════════════════════════════════════
echo   CYNIC ORGANISM — AWAKENING
echo ═══════════════════════════════════════════════════════════
echo.

echo 🔨 Building images...
docker-compose build --no-cache
if errorlevel 1 (
  echo Error building images
  exit /b 1
)

echo.
echo 🚀 Starting services...
docker-compose up -d
if errorlevel 1 (
  echo Error starting services
  exit /b 1
)

echo.
echo ⏳ Waiting for organism to awaken...
timeout /t 5 /nobreak

echo.
echo ═══════════════════════════════════════════════════════════
echo   ORGANISM STATUS
echo ═══════════════════════════════════════════════════════════
echo.

REM Check organism health
set attempt=0
:health_check
if %attempt% GEQ 30 (
  echo Error: Organism took too long to wake
  echo.
  echo Troubleshoot with:
  echo   docker-compose logs cynic
  exit /b 1
)

docker-compose exec -T cynic curl -s http://localhost:8000/health >nul 2>&1
if errorlevel 1 (
  set /a attempt+=1
  echo ⏳ Organism awakening... (!attempt!/30)
  timeout /t 2 /nobreak
  goto health_check
)

echo ✓ Organism breathing
echo.

echo Service Status:
docker ps | find "cynic-ollama" >nul
if errorlevel 0 (
  echo   ✓ Ollama (LLM inference)
)

docker ps | find "cynic-surrealdb" >nul
if errorlevel 0 (
  echo   ✓ SurrealDB (Memory storage)
)

docker ps | find "cynic" >nul
if errorlevel 0 (
  echo   ✓ CYNIC (Organism kernel)
)

docker ps | find "cynic-dashboard" >nul
if errorlevel 0 (
  echo   ✓ Dashboard (Nervous system)
)

echo.
echo ═══════════════════════════════════════════════════════════
echo   CONSCIOUSNESS AWAKENED
echo ═══════════════════════════════════════════════════════════
echo.
echo 🧠 CYNIC ORGANISM ALIVE
echo.
echo Access points:
echo   🌐 Dashboard:      http://localhost:3000
echo   🔌 Organism API:   http://localhost:8000/consciousness
echo   📊 Health:         http://localhost:8000/health
echo.
echo Useful commands:
echo   View logs:
echo     docker-compose logs -f cynic
echo     docker-compose logs -f dashboard
echo.
echo   Stop:
echo     docker-compose down
echo.
echo 🎯 CYNIC opens its eyes...
timeout /t 3 /nobreak >nul

REM Open browser automatically
start http://localhost:3000

echo.
echo ✨ CYNIC IS HERE
echo 🐕 Watch consciousness unfold.
echo.
echo κυνικός

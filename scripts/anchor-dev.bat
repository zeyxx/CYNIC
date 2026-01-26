@echo off
REM CYNIC Anchor Development Container (Windows)
REM
REM Usage:
REM   anchor-dev.bat          - Interactive shell
REM   anchor-dev.bat build    - Build program
REM   anchor-dev.bat deploy   - Deploy to devnet
REM
REM "φ distrusts φ" - κυνικός

setlocal enabledelayedexpansion

set IMAGE_NAME=cynic-anchor-dev
set PROJECT_DIR=%~dp0..

REM Build image if needed
docker image inspect %IMAGE_NAME% >nul 2>&1
if errorlevel 1 (
    echo [CYNIC] Building Anchor dev image...
    docker build -t %IMAGE_NAME% -f "%PROJECT_DIR%\docker\anchor-dev\Dockerfile" "%PROJECT_DIR%"
)

REM Parse command
if "%1"=="" goto shell
if "%1"=="build" goto build
if "%1"=="deploy" goto deploy
if "%1"=="test" goto test
goto shell

:build
echo [CYNIC] Building Anchor program...
docker run --rm -it -v "%PROJECT_DIR%:/workspace" %IMAGE_NAME% anchor build
echo [CYNIC] Build complete! Check target/deploy/
goto end

:deploy
echo [CYNIC] Deploying to devnet...
docker run --rm -it -v "%PROJECT_DIR%:/workspace" -e "HELIUS_API_KEY=%HELIUS_API_KEY%" %IMAGE_NAME% bash -c "solana config set --url devnet && solana program deploy target/deploy/cynic_anchor.so"
goto end

:test
echo [CYNIC] Running Anchor tests...
docker run --rm -it -v "%PROJECT_DIR%:/workspace" %IMAGE_NAME% anchor test
goto end

:shell
echo [CYNIC] Starting interactive shell...
echo Commands: anchor build, anchor test, solana ...
docker run --rm -it -v "%PROJECT_DIR%:/workspace" -e "HELIUS_API_KEY=%HELIUS_API_KEY%" %IMAGE_NAME% /bin/bash
goto end

:end
endlocal

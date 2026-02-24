@echo off
REM CYNIC Discord Bot Quick Start (Windows)

echo.
echo 🐕 CYNIC Discord Bot
echo ====================
echo.

REM Check if .env exists
if not exist ".env" (
    echo ❌ .env file not found!
    echo.
    echo Quick setup:
    echo 1. Copy .env.example to .env
    echo    copy .env.example .env
    echo.
    echo 2. Get Discord token from https://discord.com/developers/applications
    echo 3. Edit .env and set DISCORD_TOKEN
    echo 4. Run this script again
    echo.
    pause
    exit /b 1
)

REM Check Python
python --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Python not found. Install Python 3.10+
    pause
    exit /b 1
)

echo ✓ Python ready

REM Install dependencies if needed
python -c "import discord" >nul 2>&1
if errorlevel 1 (
    echo Installing dependencies...
    pip install -r requirements.txt
)

echo ✓ Dependencies ready
echo.
echo Starting CYNIC Discord Bot...
echo ----------------------------------------
echo.

python bot.py

pause

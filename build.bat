@echo off
setlocal enabledelayedexpansion

REM Build Project Rec and copy a single release output folder.
cd /d "%~dp0"

echo Building Project Rec in release mode...
cargo build --release
if errorlevel 1 (
    echo Build failed.
    exit /b 1
)

set OUTPUT_DIR=build
if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"
mkdir "%OUTPUT_DIR%"

necho Copying executable...
copy /y "target\release\project_rec.exe" "%OUTPUT_DIR%\project_rec.exe" >nul
if errorlevel 1 (
    echo Failed to copy executable.
    exit /b 1
)

necho Copying assets...
xcopy "assets" "%OUTPUT_DIR%\assets" /e /i /y >nul
if errorlevel 1 (
    echo Failed to copy assets.
    exit /b 1
)

necho Build output ready in %OUTPUT_DIR%\
endlocal

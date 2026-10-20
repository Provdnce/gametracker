@echo off
REM Baut GameTracker. Voraussetzung: Rust (https://rustup.rs) + Visual Studio Build Tools (C++).
where cargo >nul 2>nul || (echo Rust ist nicht installiert: https://rustup.rs & pause & exit /b 1)
cargo build --release -p gametracker || (echo Build fehlgeschlagen. & pause & exit /b 1)
copy /Y target\release\gametracker.exe GameTracker.exe >nul
echo.
echo Fertig! GameTracker.exe liegt jetzt in diesem Ordner.
start "" GameTracker.exe

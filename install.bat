@echo off
echo Installing txtCompress...
where cargo >nul 2>nul
if errorlevel 1 (
    echo.
    echo Rust is not installed. Get it from https://rustup.rs and run this again.
    pause
    exit /b 1
)
cargo install --path . --force
if errorlevel 1 (
    echo.
    echo Install failed. See the error above.
    pause
    exit /b 1
)
echo.
echo Done! Open a NEW terminal and try:  txtCompress -h
pause

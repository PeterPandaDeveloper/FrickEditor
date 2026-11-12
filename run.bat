@echo off
title FrickEditor Studio - Five Nights at Frickbear's 3
echo ==============================================================
echo Iniciando FrickEditor Studio (IDE de Modding en Rust)
echo Five Nights at Frickbear's 3 + Four Layers Down
echo ==============================================================
echo.

if exist "target\debug\frick_editor.exe" (
    start "" "target\debug\frick_editor.exe"
) else (
    echo Compilando y ejecutando FrickEditor con Cargo...
    cargo run
)

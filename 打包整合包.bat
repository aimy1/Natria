@echo off
title Natria - 自动构建与制作绿色整合包
cd /d "%~dp0"

set "PY_EXE=python"
if exist "%~dp0..\GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe" (
    set "PY_EXE=%~dp0..\GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe"
) else if exist "%~dp0GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe" (
    set "PY_EXE=%~dp0GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe"
)

echo 正在运行 Natria 整合包自动构建程序...
"%PY_EXE%" package_bundle.py
if %errorlevel% neq 0 (
    echo.
    echo [错误] 打包过程出错，请检查上方日志。
    pause
    exit /b %errorlevel%
)

echo.
pause

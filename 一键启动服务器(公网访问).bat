@echo off
chcp 936 >nul
title Natria AI 主机服务器 (全球公网访问)
cd /d "%~dp0"

echo =======================================================
echo          Natria AI 主机服务器一键启动
echo =======================================================
echo [1/2] 正在检查 Python 运行时环境与加速组件...

set PYTHON_EXE=..\GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe
if not exist "%PYTHON_EXE%" (
    set PYTHON_EXE=python
)

echo [2/2] 正在拉起 Natria 核心、GPT-SoVITS 与 Cloudflare 全球公网安全通道...
echo.

"%PYTHON_EXE%" start_server.py

pause

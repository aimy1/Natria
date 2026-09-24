@echo off
title Natria AI - 一键启动
cd /d "%~dp0"

echo ======================================================================
echo                  Natria AI 桌面智能体系统
echo            [LLM 智能对话 + GPT-SoVITS 专属声音克隆 + WebUI]
echo ======================================================================
echo.

set "NATRIA_EXE=%~dp0natria.exe"
if not exist "%NATRIA_EXE%" (
    if exist "%~dp0target\release\natria.exe" (
        set "NATRIA_EXE=%~dp0target\release\natria.exe"
    ) else if exist "%~dp0target\debug\natria.exe" (
        set "NATRIA_EXE=%~dp0target\debug\natria.exe"
    )
)

if not exist "%NATRIA_EXE%" (
    echo [错误] 未找到 natria.exe 主程序！
    echo 请确认已完成编译或解压完整。
    echo.
    pause
    exit /b 1
)

echo [1/2] 检查运行组件与环境...
if exist "%~dp0GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe" (
    echo   - GPT-SoVITS 专属声音克隆引擎 [已就绪]
) else if exist "%~dp0..\GPT-SoVITS-v2pro-20250604-nvidia50\runtime\python.exe" (
    echo   - 上一级目录 GPT-SoVITS 引擎 [已就绪]
)

echo [2/2] 正在启动 Natria 核心服务...
echo.

start "" cmd /c "ping 127.0.0.1 -n 3 >nul & start http://127.0.0.1:8300"

echo 正在监听 WebUI 访问端口 (http://127.0.0.1:8300)...
echo.
echo ----------------------------------------------------------------------
echo  浏览器将自动打开；按 Ctrl+C 可安全停止服务并释放资源。
echo ----------------------------------------------------------------------
echo.

"%NATRIA_EXE%" web
pause

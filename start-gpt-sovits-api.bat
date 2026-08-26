@echo off
chcp 65001 >nul
echo ======================================================================
echo   [Natria x GPT-SoVITS] 本地零样本声音克隆 API 服务一键启动
echo ======================================================================
echo.
set "PROJ_ROOT=%~dp0"
set "GPT_DIR=%PROJ_ROOT%GPT-SoVITS-v2pro-20250604-nvidia50"

if not exist "%GPT_DIR%\runtime\python.exe" (
    echo [错误] 未在根目录下找到 GPT-SoVITS-v2pro-20250604-nvidia50 整合包！
    echo 请确认整合包文件夹已放置在项目根目录。
    pause
    exit /b 1
)

cd /d "%GPT_DIR%"
set "PATH=%GPT_DIR%\runtime;%PATH%"

echo 正在启动 GPT-SoVITS API 服务 (http://127.0.0.1:9880)...
echo.
runtime\python.exe -I api_v2.py -a 127.0.0.1 -p 9880 -c GPT_SoVITS/configs/tts_infer.yaml
pause

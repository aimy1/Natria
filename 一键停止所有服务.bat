@echo off
chcp 936 >nul
title 停止 Natria 所有服务
cd /d "%~dp0"

echo =======================================================
echo          正在停止 Natria 所有服务与后台进程...
echo =======================================================
echo.

taskkill /f /im natria.exe >nul 2>&1
taskkill /f /im cloudflared.exe >nul 2>&1

echo   [+] 已停止 Natria 核心服务
echo   [+] 已停止 Cloudflare 隧道进程

ping 127.0.0.1 -n 2 >nul

echo.
echo =======================================================
echo   全部服务已安全关闭！
echo =======================================================
echo.
ping 127.0.0.1 -n 3 >nul

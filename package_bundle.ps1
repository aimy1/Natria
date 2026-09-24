# Natria Windows 一体化绿色整合包自动构建与打包脚本
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectDir = $ScriptDir
$ParentDir = Split-Path -Parent $ProjectDir
$TargetReleaseDir = Join-Path $ParentDir "Natria-v0.4.5-AllInOne-Windows-RTX"

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "       Natria v0.4.5 Windows RTX 一体化绿色整合包构建器" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "[1/5] 正在以 Release 模式编译 Natria 核心主程序 (最大优化)..." -ForegroundColor Yellow

Push-Location $ProjectDir
try {
    # 停止当前可能占用的 natria 进程
    Get-Process natria -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo release 编译失败！"
    }
} finally {
    Pop-Location
}

Write-Host "  ✓ 核心二进制编译完成：target\release\natria.exe" -ForegroundColor Green
Write-Host ""

Write-Host "[2/5] 准备整合包目标输出目录..." -ForegroundColor Yellow
Write-Host "  目标路径: $TargetReleaseDir" -ForegroundColor Gray

if (Test-Path $TargetReleaseDir) {
    Write-Host "  清理旧的输出文件..." -ForegroundColor Gray
    # 保留大型 models 与 python runtime 避免重复长耗时复制，更新其余文件
} else {
    New-Item -ItemType Directory -Path $TargetReleaseDir -Force | Out-Null
}

$SubDirs = @("bin", "models", "voices", "web", "assets")
foreach ($sub in $SubDirs) {
    $p = Join-Path $TargetReleaseDir $sub
    if (-not (Test-Path $p)) {
        New-Item -ItemType Directory -Path $p -Force | Out-Null
    }
}

Write-Host "[3/5] 组装核心资产与依赖库..." -ForegroundColor Yellow

# 复制主可执行程序
Copy-Item (Join-Path $ProjectDir "target\release\natria.exe") $TargetReleaseDir -Force
Write-Host "  ✓ 复制 natria.exe" -ForegroundColor Green

# 复制 bin 目录（llama.cpp / CUDA 加速库）
$binFiles = Get-ChildItem (Join-Path $ProjectDir "bin") -File
foreach ($f in $binFiles) {
    Copy-Item $f.FullName (Join-Path $TargetReleaseDir "bin") -Force
}
Write-Host "  ✓ 复制 bin/ ($($binFiles.Count) 个 CUDA / llama 加速动态库)" -ForegroundColor Green

# 复制 web 资产
Copy-Item (Join-Path $ProjectDir "web\*") (Join-Path $TargetReleaseDir "web") -Recurse -Force
Write-Host "  ✓ 复制 web/ 前端界面资产" -ForegroundColor Green

# 复制 voices 音频
$voiceFiles = Get-ChildItem (Join-Path $ProjectDir "voices") -File
foreach ($f in $voiceFiles) {
    Copy-Item $f.FullName (Join-Path $TargetReleaseDir "voices") -Force
}
Write-Host "  ✓ 复制 voices/ ($($voiceFiles.Count) 个参考声线切片)" -ForegroundColor Green

# 复制 models (大模型文件)
$modelFiles = Get-ChildItem (Join-Path $ProjectDir "models") -File
foreach ($f in $modelFiles) {
    $dest = Join-Path $TargetReleaseDir "models\$($f.Name)"
    if (-not (Test-Path $dest)) {
        Write-Host "  -> 复制模型 $($f.Name) (可能需要十几秒)..." -ForegroundColor Gray
        Copy-Item $f.FullName $dest -Force
    } else {
        Write-Host "  ✓ 模型已存在，跳过冗余拷贝: $($f.Name)" -ForegroundColor Green
    }
}

# 复制一键启动脚本与说明文档
Copy-Item (Join-Path $ProjectDir "一键启动Natria.bat") $TargetReleaseDir -Force
Copy-Item (Join-Path $ProjectDir "一键停止所有服务.bat") $TargetReleaseDir -Force
Copy-Item (Join-Path $ProjectDir "使用说明与快速上手.txt") $TargetReleaseDir -Force
Write-Host "  ✓ 复制一键启动/停止脚本与使用文档" -ForegroundColor Green

Write-Host ""
Write-Host "[4/5] 关联整合 GPT-SoVITS 专属声音克隆引擎..." -ForegroundColor Yellow
$SovitsSrc = Join-Path $ParentDir "GPT-SoVITS-v2pro-20250604-nvidia50"
$SovitsDest = Join-Path $TargetReleaseDir "GPT-SoVITS-v2pro-20250604-nvidia50"

if (Test-Path $SovitsSrc) {
    if (-not (Test-Path $SovitsDest)) {
        Write-Host "  正在复制 GPT-SoVITS 运行时与模型权重 (包含 Python 嵌入式环境)..." -ForegroundColor Gray
        Copy-Item $SovitsSrc $SovitsDest -Recurse -Force
        Write-Host "  ✓ GPT-SoVITS 运行时与小盐专属权重已完整集成！" -ForegroundColor Green
    } else {
        Write-Host "  ✓ 目标目录已包含 GPT-SoVITS 运行时，更新权重与脚本..." -ForegroundColor Green
        Copy-Item (Join-Path $SovitsSrc "weight.json") $SovitsDest -Force -ErrorAction SilentlyContinue
        Copy-Item (Join-Path $SovitsSrc "api_v2.py") $SovitsDest -Force -ErrorAction SilentlyContinue
        Copy-Item (Join-Path $SovitsSrc "GPT_SoVITS\configs\tts_infer.yaml") (Join-Path $SovitsDest "GPT_SoVITS\configs") -Force -ErrorAction SilentlyContinue
    }
} else {
    Write-Host "  ! 提示: 未在上一级目录检测到 GPT-SoVITS 整合包源文件夹，跳过合并。" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[5/5] 整合包构建完成！" -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "整合包输出目录：" -ForegroundColor Green
Write-Host "  $TargetReleaseDir" -ForegroundColor White
Write-Host ""
Write-Host "使用方法：" -ForegroundColor Yellow
Write-Host "  直接将该文件夹打包压缩或复制至任意 Windows 电脑，双击【一键启动Natria.bat】即可全自动运行！" -ForegroundColor Gray
Write-Host "======================================================================" -ForegroundColor Cyan

# -*- coding: utf-8 -*-
import os
import sys
import shutil
import subprocess
import time

def main():
    print("=" * 70)
    print("       Natria v0.4.5 Windows RTX 一体化绿色整合包构建器")
    print("            [Natria 核心 + GPT-SoVITS 专属声音克隆]")
    print("=" * 70)

    proj_dir = os.path.abspath(os.path.dirname(__file__))
    parent_dir = os.path.dirname(proj_dir)
    target_dir = os.path.join(parent_dir, "Natria-v0.4.5-AllInOne-Windows-RTX")

    print(f"\n[1/4] 检查/编译 Natria Release 核心主程序...")
    if sys.platform == "win32":
        subprocess.run(["taskkill", "/F", "/IM", "natria.exe"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    release_exe = os.path.join(proj_dir, "target", "release", "natria.exe")
    if not os.path.exists(release_exe):
        print("  -> 执行 cargo build --release...")
        ret = subprocess.run(["cargo", "build", "--release"], cwd=proj_dir)
        if ret.returncode != 0:
            print("[错误] cargo build --release 编译失败！")
            sys.exit(1)

    print(f"  [+] 核心二进制就绪: {release_exe}")

    print(f"\n[2/4] 准备整合包目标输出目录: {target_dir}")
    os.makedirs(target_dir, exist_ok=True)
    for sub in ["voices", "web", "assets"]:
        os.makedirs(os.path.join(target_dir, sub), exist_ok=True)

    # 清理遗留的 bin 和 models 目录（若存在）
    for old_dir in ["bin", "models"]:
        old_p = os.path.join(target_dir, old_dir)
        if os.path.exists(old_p):
            shutil.rmtree(old_p, ignore_errors=True)

    print("\n[3/4] 组装核心资产与前端界面...")
    # 1. 复制 natria.exe
    shutil.copy2(release_exe, os.path.join(target_dir, "natria.exe"))
    print("  [+] 复制 natria.exe")

    # 2. 复制 web/ 前端界面资产
    web_src = os.path.join(proj_dir, "web")
    web_dst = os.path.join(target_dir, "web")
    if os.path.exists(web_src):
        shutil.copytree(web_src, web_dst, dirs_exist_ok=True)
        print("  [+] 复制 web/ 静态界面资产")

    # 3. 复制 voices/ 参考音频（自动同步 audio_dataset 内的所有切片与样本）
    sovits_src = os.path.join(parent_dir, "GPT-SoVITS-v2pro-20250604-nvidia50")
    dataset_src = os.path.join(sovits_src, "audio_dataset")
    voices_src = os.path.join(proj_dir, "voices")
    voices_dst = os.path.join(target_dir, "voices")
    os.makedirs(voices_src, exist_ok=True)
    os.makedirs(voices_dst, exist_ok=True)

    if os.path.exists(dataset_src):
        for item in os.listdir(dataset_src):
            if item.lower().endswith(".wav") or item.lower().endswith(".list"):
                shutil.copy2(os.path.join(dataset_src, item), os.path.join(voices_src, item))
    if os.path.exists(sovits_src):
        for item in os.listdir(sovits_src):
            if item.lower().endswith(".wav"):
                shutil.copy2(os.path.join(sovits_src, item), os.path.join(voices_src, item))

    v_count = 0
    for item in os.listdir(voices_src):
        s = os.path.join(voices_src, item)
        d = os.path.join(voices_dst, item)
        if os.path.isfile(s):
            shutil.copy2(s, d)
            v_count += 1
    print(f"  [+] 复制 voices/ ({v_count} 个声线切片与参考音频)")

    # 4. 复制一键启动脚本与说明文档
    for doc in ["一键启动Natria.bat", "一键停止所有服务.bat", "使用说明与快速上手.txt"]:
        src_doc = os.path.join(proj_dir, doc)
        if os.path.exists(src_doc):
            shutil.copy2(src_doc, os.path.join(target_dir, doc))
            print(f"  [+] 复制 {doc}")

    print("\n[4/4] 关联整合 GPT-SoVITS 专属声音克隆引擎...")
    sovits_src = os.path.join(parent_dir, "GPT-SoVITS-v2pro-20250604-nvidia50")
    sovits_dst = os.path.join(target_dir, "GPT-SoVITS-v2pro-20250604-nvidia50")

    if os.path.exists(sovits_src):
        if not os.path.exists(sovits_dst):
            print("  [->] 正在复制 GPT-SoVITS 运行时与模型权重 (包含 Python 嵌入式环境)...")
            shutil.copytree(sovits_src, sovits_dst)
            print("  [+] GPT-SoVITS 运行时与小盐专属权重已完整集成！")
        else:
            print("  [+] 目标目录已包含 GPT-SoVITS 运行时，更新权重与脚本...")
            for f in ["weight.json", "api_v2.py"]:
                sf = os.path.join(sovits_src, f)
                if os.path.exists(sf):
                    shutil.copy2(sf, os.path.join(sovits_dst, f))
            cfg_src = os.path.join(sovits_src, "GPT_SoVITS", "configs", "tts_infer.yaml")
            cfg_dst = os.path.join(sovits_dst, "GPT_SoVITS", "configs", "tts_infer.yaml")
            if os.path.exists(cfg_src):
                os.makedirs(os.path.dirname(cfg_dst), exist_ok=True)
                shutil.copy2(cfg_src, cfg_dst)
    else:
        print("  [!] 提示: 未在上一级目录找到 GPT-SoVITS 整合包文件夹")

    print("\n" + "=" * 70)
    print("                  [+] 绿色整合包构建完成！")
    print("=" * 70)
    print(f"整合包输出路径: {target_dir}")
    print("\n使用方法: 进入该目录，双击【一键启动Natria.bat】即可开箱即用！")
    print("=" * 70)

if __name__ == "__main__":
    main()

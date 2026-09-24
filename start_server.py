import subprocess
import time
import re
import os
import sys
import socket

# Force UTF-8 safe output
if hasattr(sys.stdout, 'reconfigure'):
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='ignore')
    except:
        pass

def get_local_ips():
    ips = []
    try:
        host_name = socket.gethostname()
        for ip in socket.gethostbyname_ex(host_name)[2]:
            if not ip.startswith("127."):
                ips.append(ip)
    except:
        pass
    return ips or ["127.0.0.1"]

def main():
    proj_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(proj_dir)
    
    cf_path = os.path.join(proj_dir, "cloudflared.exe")
    candidates = [
        os.path.join(proj_dir, "target", "release", "natria.exe"),
        os.path.join(proj_dir, "target", "debug", "natria.exe"),
        os.path.join(proj_dir, "natria.exe"),
    ]
    natria_exe = candidates[0]
    newest_mtime = -1
    for c in candidates:
        if os.path.exists(c):
            mt = os.path.getmtime(c)
            if mt > newest_mtime:
                newest_mtime = mt
                natria_exe = c
        
    log_path = os.path.join(proj_dir, "cloudflared.log")
    if os.path.exists(log_path):
        try:
            os.remove(log_path)
        except:
            pass

    print("\n" + "="*65)
    print("      >>> NATRIA AI HOST SERVER (Cloudflare Tunnel) <<<")
    print("="*65)
    print("[1/3] 正在启动 Natria 核心服务与 GPT-SoVITS 语音引擎...")

    # Start Natria if not already listening
    natria_proc = None
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(1)
        res = s.connect_ex(('127.0.0.1', 8300))
        s.close()
        if res != 0:
            # Not running, start it
            natria_proc = subprocess.Popen([natria_exe, "web"], cwd=proj_dir)
            time.sleep(2)
        else:
            print("  [+] Natria 服务已在后台就绪 (端口 8300)")
    except Exception as e:
        print(f"  [!] 启动 Natria 提示: {e}")

    print("[2/3] 正在连接 Cloudflare 全球边缘网络，申请专属公网安全通道...")
    cf_proc = subprocess.Popen(
        [cf_path, "tunnel", "--url", "http://127.0.0.1:8300", "--logfile", log_path],
        cwd=proj_dir,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    public_url = None
    for _ in range(30):
        time.sleep(1)
        if os.path.exists(log_path):
            try:
                with open(log_path, "r", encoding="utf-8", errors="ignore") as f:
                    content = f.read()
                    m = re.search(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com", content)
                    if m:
                        public_url = m.group(0)
                        break
            except:
                pass

    print("[3/3] 服务器已成功上线！\n")
    print("="*65)
    print("  [*] 你的电脑已成功变身为专属 Natria AI 服务器！")
    print("="*65)
    print(f"\n  >>> 全球外网访问 (手机/平板/异地网络直接打开):")
    if public_url:
        print(f"      {public_url}")
    else:
        print(f"      (隧道正在建立中，请稍候查看 cloudflared.log)")
    
    local_ips = get_local_ips()
    print(f"\n  >>> 同一局域网/Wi-Fi 访问:")
    for ip in local_ips:
        print(f"      http://{ip}:8300")
    print(f"  >>> 本机访问:\n      http://127.0.0.1:8300")
    print("\n" + "="*65)
    print("  [i] 提示: 保持本窗口运行，服务器将持续对外提供服务。")
    print("  [!] 随时按 Ctrl+C 或双击【一键停止所有服务.bat】即可关闭。")
    print("="*65 + "\n")

    try:
        while True:
            time.sleep(2)
            if cf_proc.poll() is not None:
                print("[!] 隧道连接意外断开，正在尝试重新连接...")
                cf_proc = subprocess.Popen(
                    [cf_path, "tunnel", "--url", "http://127.0.0.1:8300", "--logfile", log_path],
                    cwd=proj_dir,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL
                )
    except KeyboardInterrupt:
        print("\n正在停止服务器与隧道连接...")
        try:
            cf_proc.terminate()
        except:
            pass
        print("已退出服务器守护。")

if __name__ == "__main__":
    main()

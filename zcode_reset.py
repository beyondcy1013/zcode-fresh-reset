#!/usr/bin/env python3
"""
ZCode Fresh Reset & Session Sanitizer
====================================
安全清理 ZCode 客户端本地登录状态、会话与套餐权益缓存，
重置客户端为“新机首次启动”状态，方便接入新账号或重新拉取权益。
"""

import argparse
import datetime
import os
import shutil
import subprocess
import sys
from pathlib import Path

DEFAULT_HOME = Path(os.environ.get("HOME", "/root"))

CANDIDATE_PATHS = [
    # 登录凭据与套餐缓存 (核心)
    ("credentials", "v2/credentials.json", "~/.zcode/v2/credentials.json"),
    ("plan_cache", "v2/coding-plan-cache.json", "~/.zcode/v2/coding-plan-cache.json"),
    ("telemetry", "v2/telemetry-state.json", "~/.zcode/v2/telemetry-state.json"),
    ("auth_dir", "auth", "~/.zcode/auth"),
    # Electron 会话与网页 Cookie/Storage
    ("session_cookies", "session/Cookies", "~/.config/ZCode/session/Cookies"),
    ("session_storage", "session/Local Storage", "~/.config/ZCode/session/Local Storage"),
    ("session_indexeddb", "session/IndexedDB", "~/.config/ZCode/session/IndexedDB"),
    ("session_full", "session", "~/.config/ZCode/session"),
    # 设备标识与埋点
    ("electron_store", "rum-electron-store", "~/.config/ZCode/rum-electron-store"),
    ("updater_id", ".updaterId", "~/.config/ZCode/.updaterId"),
]

def resolve_path(pattern_str: str) -> Path:
    expanded = os.path.expanduser(pattern_str)
    return Path(expanded)

def check_zcode_running() -> bool:
    try:
        output = subprocess.check_output(["pgrep", "-i", "-f", "zcode"], text=True)
        return bool(output.strip())
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False

def inspect_status():
    print("=" * 60)
    print("  ZCode 本地记录与状态检测")
    print("=" * 60)
    
    is_running = check_zcode_running()
    print(f"[*] 进程状态: {'[警告] ZCode 正在运行 (请先关闭客户端)' if is_running else '[正常] ZCode 未运行'}")
    print("\n[+] 检查关键本地记录路径:")
    
    found_any = False
    for tag, rel, p_str in CANDIDATE_PATHS:
        p = resolve_path(p_str)
        if p.exists():
            found_any = True
            size_info = ""
            if p.is_file():
                size_info = f"({p.stat().st_size} bytes)"
            elif p.is_dir():
                file_count = len(list(p.rglob("*")))
                size_info = f"({file_count} files)"
            print(f"  [√ 存在] {tag:20} -> {p} {size_info}")
        else:
            print(f"  [- 缺失] {tag:20} -> {p}")
            
    if not found_any:
        print("\n提示: 当前环境下未检测到相关 ZCode 本地缓存或已被清理。")
    print("=" * 60)

def backup_state(backup_dir: Path):
    backup_dir.mkdir(parents=True, exist_ok=True)
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    target_bak = backup_dir / f"zcode_backup_{ts}"
    target_bak.mkdir(parents=True, exist_ok=True)
    
    print(f"[*] 正在备份现有配置到: {target_bak}")
    copied = 0
    for tag, rel, p_str in CANDIDATE_PATHS:
        p = resolve_path(p_str)
        if p.exists():
            dest = target_bak / tag
            if p.is_file():
                shutil.copy2(p, dest)
            elif p.is_dir():
                shutil.copytree(p, dest, dirs_exist_ok=True)
            copied += 1
    print(f"[√] 备份完成，已保存 {copied} 个关键项目。")
    return target_bak

def clean_state(safe_mode: bool = False, no_backup: bool = False, backup_path: str = None):
    if check_zcode_running():
        print("[!] 错误: 检测到 ZCode 进程正在运行，请先完全退出 ZCode 后再执行清理！")
        sys.exit(1)
        
    if not no_backup:
        bak_root = Path(backup_path) if backup_path else (DEFAULT_HOME / ".zcode" / "reset_backups")
        backup_state(bak_root)
        
    print("\n[*] 开始清理本地登录及权益缓存...")
    
    # 待删除的目标
    if safe_mode:
        # 安全模式：仅清理套餐缓存和会话 Cookie，保留其他设置
        targets = [
            ("plan_cache", "~/.zcode/v2/coding-plan-cache.json"),
            ("session_cookies", "~/.config/ZCode/session/Cookies"),
            ("telemetry", "~/.zcode/v2/telemetry-state.json"),
        ]
    else:
        # 全量模式：重置为新装状态
        targets = [
            ("credentials", "~/.zcode/v2/credentials.json"),
            ("plan_cache", "~/.zcode/v2/coding-plan-cache.json"),
            ("telemetry", "~/.zcode/v2/telemetry-state.json"),
            ("auth_dir", "~/.zcode/auth"),
            ("session_full", "~/.config/ZCode/session"),
            ("electron_store", "~/.config/ZCode/rum-electron-store"),
            ("updater_id", "~/.config/ZCode/.updaterId"),
        ]
        
    cleaned = 0
    for tag, p_str in targets:
        p = resolve_path(p_str)
        if p.exists():
            try:
                if p.is_file() or p.is_symlink():
                    p.unlink()
                elif p.is_dir():
                    shutil.rmtree(p)
                print(f"  [√ 已清理] {tag} ({p})")
                cleaned += 1
            except Exception as e:
                print(f"  [!] 清理失败 {tag} ({p}): {e}")
        else:
            print(f"  [- 跳过] {tag} 不存在")
            
    print(f"\n[√] 清理完成！共处理 {cleaned} 项。")
    print("-" * 60)
    print("【重要提示】")
    print("1. 本地客户端现已处于新机/新用户初始状态。")
    print("2. 重新启动 ZCode 时，客户端将重新引导登录并请求权益。")
    print("3. 若需要领取新用户 Flash 免费套餐，请登录【未领取过该福利的新账号】。")
    print("   (注: 免费套餐资格受服务端账号与设备判定，已领过的旧账号服务端仍会识别)。")
    print("-" * 60)

def main():
    parser = argparse.ArgumentParser(
        description="ZCode Fresh Reset & Session Sanitizer - 重置 ZCode 客户端本地状态工具"
    )
    subparsers = parser.add_subparsers(dest="action", help="执行操作")
    
    # inspect
    subparsers.add_parser("inspect", help="检查当前本地 ZCode 记录与缓存状态")
    
    # clean
    clean_parser = subparsers.add_parser("clean", help="执行本地清理与重置")
    clean_parser.add_argument(
        "--safe",
        action="store_true",
        help="安全模式：仅清理套餐缓存和会话，不删除已存凭据",
    )
    clean_parser.add_argument(
        "--no-backup",
        action="store_true",
        help="不创建自动备份（不推荐）",
    )
    clean_parser.add_argument(
        "--backup-dir",
        type=str,
        default="",
        help="自定义备份保存目录",
    )
    
    # backup
    bak_parser = subparsers.add_parser("backup", help="仅备份现有登录与缓存文件")
    bak_parser.add_argument(
        "--backup-dir",
        type=str,
        default="",
        help="自定义备份保存目录",
    )
    
    args = parser.parse_args()
    
    if args.action == "inspect":
        inspect_status()
    elif args.action == "clean":
        clean_state(
            safe_mode=args.safe,
            no_backup=args.no_backup,
            backup_path=args.backup_dir or None,
        )
    elif args.action == "backup":
        bak_root = Path(args.backup_dir) if args.backup_dir else (DEFAULT_HOME / ".zcode" / "reset_backups")
        backup_state(bak_root)
    else:
        parser.print_help()
        print("\n当前状态概览:")
        inspect_status()

if __name__ == "__main__":
    main()

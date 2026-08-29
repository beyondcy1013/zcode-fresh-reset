#![cfg_attr(windows, windows_subsystem = "windows")]

use semver::Version;
use serde::Deserialize;
use std::thread;
use std::time::Duration;
use std::{
    env,
    ffi::OsStr,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

mod accounts;
mod gui;

#[derive(Clone, Copy)]
enum Root {
    UserProfile,
    AppData,
}

#[derive(Clone, Copy)]
struct Candidate {
    tag: &'static str,
    root: Root,
    relative: &'static str,
}

const CANDIDATES: &[Candidate] = &[
    Candidate {
        tag: "credentials",
        root: Root::UserProfile,
        relative: ".zcode/v2/credentials.json",
    },
    Candidate {
        tag: "plan_cache",
        root: Root::UserProfile,
        relative: ".zcode/v2/coding-plan-cache.json",
    },
    Candidate {
        tag: "telemetry",
        root: Root::UserProfile,
        relative: ".zcode/v2/telemetry-state.json",
    },
    Candidate {
        tag: "auth_dir",
        root: Root::UserProfile,
        relative: ".zcode/auth",
    },
    Candidate {
        tag: "session_cookies",
        root: Root::AppData,
        relative: "ZCode/session/Cookies",
    },
    Candidate {
        tag: "session_storage",
        root: Root::AppData,
        relative: "ZCode/session/Local Storage",
    },
    Candidate {
        tag: "session_indexeddb",
        root: Root::AppData,
        relative: "ZCode/session/IndexedDB",
    },
    Candidate {
        tag: "session_full",
        root: Root::AppData,
        relative: "ZCode/session",
    },
    Candidate {
        tag: "electron_store",
        root: Root::AppData,
        relative: "ZCode/rum-electron-store",
    },
    Candidate {
        tag: "updater_id",
        root: Root::AppData,
        relative: "ZCode/.updaterId",
    },
];
const SAFE_TAGS: &[&str] = &["plan_cache", "session_cookies", "telemetry"];
const FULL_TAGS: &[&str] = &[
    "credentials",
    "plan_cache",
    "telemetry",
    "auth_dir",
    "session_full",
    "electron_store",
    "updater_id",
];

#[derive(Clone)]
struct Roots {
    user_profile: PathBuf,
    app_data: PathBuf,
}

impl Roots {
    fn detect() -> Result<Self, String> {
        let user_profile = env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from)
            .ok_or("未找到 USERPROFILE 环境变量")?;
        let app_data = env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| user_profile.join("AppData").join("Roaming"));
        Ok(Self {
            user_profile,
            app_data,
        })
    }
    fn resolve(&self, candidate: Candidate) -> PathBuf {
        let root = match candidate.root {
            Root::UserProfile => &self.user_profile,
            Root::AppData => &self.app_data,
        };
        candidate
            .relative
            .split('/')
            .fold(root.clone(), |path, part| path.join(part))
    }
}

#[derive(Default)]
struct CleanOptions {
    safe: bool,
    no_backup: bool,
    backup_dir: Option<PathBuf>,
}
enum Action {
    Gui,
    InteractiveCli,
    Inspect,
    Backup(Option<PathBuf>),
    Clean(CleanOptions),
    Help,
    Version,
    CheckUpdate,
}

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Zh,
    En,
}

fn lang() -> Lang {
    match env::var("ZCODE_LANG")
        .ok()
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("en") | Some("en-us") | Some("english") => Lang::En,
        _ => Lang::Zh,
    }
}

fn tr(zh: &str, en: &str) -> String {
    if lang() == Lang::En {
        en.into()
    } else {
        zh.into()
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Action, String> {
    let mut args = args.into_iter();
    let Some(action) = args.next() else {
        return Ok(Action::Gui);
    };
    if ["--help", "-h"].contains(&action.as_str()) {
        return Ok(Action::Help);
    }
    if ["--version", "-V"].contains(&action.as_str()) {
        return Ok(Action::Version);
    }
    if action == "--check-update" {
        return Ok(Action::CheckUpdate);
    }
    let mut options = CleanOptions::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--safe" if action == "clean" => options.safe = true,
            "--no-backup" if action == "clean" => options.no_backup = true,
            "--backup-dir" if action == "clean" || action == "backup" => {
                options.backup_dir = Some(PathBuf::from(
                    args.next().ok_or("--backup-dir 缺少目录参数")?,
                ));
            }
            _ => return Err(format!("未知参数: {arg}")),
        }
    }
    match action.as_str() {
        "interactive" => Ok(Action::InteractiveCli),
        "inspect" => Ok(Action::Inspect),
        "backup" => Ok(Action::Backup(options.backup_dir)),
        "clean" => Ok(Action::Clean(options)),
        _ => Err(format!("未知命令: {action}")),
    }
}

fn zcode_running() -> bool {
    #[cfg(windows)]
    return Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq ZCode.exe", "/FO", "CSV", "/NH"])
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .to_ascii_lowercase()
                .contains("zcode.exe")
        })
        .unwrap_or(false);
    #[cfg(not(windows))]
    return Command::new("pgrep")
        .args(["-i", "-x", "zcode"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
}

#[derive(Deserialize)]
struct UpdateManifest {
    version: String,
    url: String,
    sha256: Option<String>,
}

const DEFAULT_UPDATE_MANIFEST_URL: &str = "https://github.com/beyondcy1013/zcode-fresh-reset/releases/latest/download/update-manifest.json";

fn fetch_update() -> Result<Option<UpdateManifest>, String> {
    let url = env::var_os("ZCODE_UPDATE_MANIFEST_URL")
        .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.into());
    let output = Command::new("curl")
        .args(["-fsSL", "--max-time", "5"])
        .arg(url)
        .output();
    let output = output.map_err(|error| format!("curl: {error}"))?;
    if !output.status.success() {
        return Err(tr("更新服务器暂时不可用", "update server is unavailable"));
    }
    let manifest: UpdateManifest =
        serde_json::from_slice(&output.stdout).map_err(|error| format!("manifest: {error}"))?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| error.to_string())?;
    let remote = Version::parse(&manifest.version).map_err(|error| error.to_string())?;
    Ok((remote > current).then_some(manifest))
}

fn check_update() {
    match fetch_update() {
        Ok(Some(update)) => println!(
            "{}",
            tr(
                &format!(
                    "[更新] 发现新版本 {}，当前版本 {}。",
                    update.version,
                    env!("CARGO_PKG_VERSION")
                ),
                &format!(
                    "[Update] Version {} is available (current {}).",
                    update.version,
                    env!("CARGO_PKG_VERSION")
                )
            )
        ),
        Ok(None) => println!(
            "{}",
            tr(
                "[更新] 当前已是最新版本。",
                "[Update] You are using the latest version."
            )
        ),
        Err(error) => println!(
            "{}: {error}",
            tr("[更新] 检查失败", "[Update] Check failed")
        ),
    }
}

fn install_update(update: &UpdateManifest) -> Result<(), String> {
    let current = env::current_exe().map_err(|error| error.to_string())?;
    let next = current.with_extension("exe.new");
    println!(
        "{}",
        tr(
            "[更新] 正在下载新版本...",
            "[Update] Downloading new version..."
        )
    );
    let status = Command::new("curl")
        .args(["-fL", "--retry", "2", "-o"])
        .arg(&next)
        .arg(&update.url)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(tr("下载更新失败", "update download failed"));
    }
    if let Some(expected) = &update.sha256 {
        let output = Command::new("certutil")
            .arg("-hashfile")
            .arg(&next)
            .arg("SHA256")
            .output()
            .map_err(|error| error.to_string())?;
        let hash_output = String::from_utf8_lossy(&output.stdout);
        let actual = hash_output
            .lines()
            .map(str::trim)
            .find(|line| line.len() == 64)
            .unwrap_or("")
            .to_string();
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&next);
            return Err(tr(
                "更新文件校验失败",
                "update checksum verification failed",
            ));
        }
    }
    #[cfg(windows)]
    {
        let script = current.with_extension("update.cmd");
        let body = format!("@echo off\r\ntimeout /t 2 /nobreak >nul\r\nmove /y \"{}\" \"{}\" >nul\r\nstart \"\" \"{}\"\r\ndel \"%~f0\"\r\n", next.display(), current.display(), current.display());
        fs::write(&script, body).map_err(|error| error.to_string())?;
        Command::new("cmd")
            .args(["/C", "start", "", &script.to_string_lossy()])
            .spawn()
            .map_err(|error| error.to_string())?;
    }
    println!(
        "{}",
        tr(
            "[更新] 下载和校验完成，即将替换并重启。",
            "[Update] Downloaded and verified. Restarting with the new version."
        )
    );
    Ok(())
}

fn terminate_zcode() -> Result<(), String> {
    println!("[步骤 2/4] 正在强行结束 ZCode 进程...");
    #[cfg(windows)]
    let result = Command::new("taskkill")
        .args(["/F", "/T", "/IM", "ZCode.exe"])
        .output();
    #[cfg(not(windows))]
    let result = Command::new("pkill")
        .args(["-TERM", "-f", "zcode"])
        .output();
    match result {
        Ok(output) if output.status.success() || !zcode_running() => {
            for _ in 0..10 {
                if !zcode_running() {
                    println!("[完成] ZCode 进程已结束。");
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(500));
            }
            Err("强行结束后仍检测到 ZCode 进程，请手动结束后重试".into())
        }
        Ok(output) => Err(format!(
            "结束 ZCode 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => Err(format!("调用进程结束命令失败: {error}")),
    }
}

fn count_entries(path: &Path) -> io::Result<u64> {
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        count += 1;
        if entry.file_type()?.is_dir() {
            count += count_entries(&entry.path())?;
        }
    }
    Ok(count)
}

fn inspect(roots: &Roots) -> io::Result<()> {
    println!("============================================================\n  ZCode 本地记录与状态检测\n============================================================");
    println!(
        "[*] 进程状态: {}",
        if zcode_running() {
            "[警告] ZCode 正在运行（请先关闭客户端）"
        } else {
            "[正常] ZCode 未运行"
        }
    );
    println!("\n[+] 检查关键本地记录路径:");
    let mut found = false;
    for candidate in CANDIDATES {
        let path = roots.resolve(*candidate);
        if path.is_file() {
            found = true;
            println!(
                "  [存在] {:20} -> {} ({} bytes)",
                candidate.tag,
                path.display(),
                fs::metadata(&path)?.len()
            );
        } else if path.is_dir() {
            found = true;
            println!(
                "  [存在] {:20} -> {} ({} entries)",
                candidate.tag,
                path.display(),
                count_entries(&path)?
            );
        } else {
            println!("  [缺失] {:20} -> {}", candidate.tag, path.display());
        }
    }
    if !found {
        println!("\n提示: 当前环境下未检测到相关 ZCode 本地缓存或已被清理。");
    }
    println!("============================================================");
    Ok(())
}

fn copy_path(source: &Path, destination: &Path) -> io::Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        println!(
            "  [已备份] {} -> {}",
            source.display(),
            destination.display()
        );
    }
    Ok(())
}

fn candidate_by_tag(tag: &str) -> Candidate {
    *CANDIDATES
        .iter()
        .find(|candidate| candidate.tag == tag)
        .expect("known candidate tag")
}

fn remove_path(path: &Path) -> io::Result<()> {
    if path.is_dir() && !path.is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn backup(roots: &Roots, backup_root: &Path) -> io::Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let target = backup_root.join(format!("zcode_backup_{stamp}"));
    fs::create_dir_all(&target)?;
    println!("[*] 正在备份现有配置到: {}", target.display());
    let mut copied = 0;
    for candidate in CANDIDATES {
        let source = roots.resolve(*candidate);
        if source.exists() {
            copy_path(&source, &target.join(candidate.tag))?;
            copied += 1;
        }
    }
    println!("[完成] 备份完成，已保存 {copied} 个关键项目。");
    Ok(target)
}

fn clean(roots: &Roots, options: CleanOptions) -> Result<(), String> {
    if zcode_running() {
        return Err("检测到 ZCode 进程正在运行，请先完全退出 ZCode 后再执行清理".into());
    }
    if !options.no_backup {
        let root = options
            .backup_dir
            .unwrap_or_else(|| roots.user_profile.join(".zcode").join("reset_backups"));
        backup(roots, &root).map_err(|e| format!("备份失败，已停止清理: {e}"))?;
    }
    println!("\n[*] 开始清理本地登录及权益缓存...");
    let selected = if options.safe { SAFE_TAGS } else { FULL_TAGS };
    let (mut cleaned, mut failures) = (0, 0);
    for tag in selected {
        let candidate = CANDIDATES
            .iter()
            .find(|c| c.tag == *tag)
            .expect("known tag");
        let path = roots.resolve(*candidate);
        if !path.exists() {
            println!("  [跳过] {tag} 不存在");
            continue;
        }
        let entries = collect_entries(&path)
            .map_err(|error| format!("读取待清理路径失败 {}: {error}", path.display()))?;
        let result = if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Ok(()) => {
                for entry in entries {
                    println!(
                        "  [{}] {}",
                        if entry.is_dir {
                            "已清除目录"
                        } else {
                            "已清除文件"
                        },
                        entry.path.display()
                    );
                    cleaned += 1;
                }
            }
            Err(error) => {
                eprintln!("  [清理失败] {tag} ({}): {error}", path.display());
                failures += 1;
            }
        }
    }
    println!("\n[步骤 4/4] 清理完成，共清除 {cleaned} 个文件或目录。");
    if failures > 0 {
        Err(format!("有 {failures} 项清理失败"))
    } else {
        Ok(())
    }
}

struct PathEntry {
    path: PathBuf,
    is_dir: bool,
}

fn collect_entries(path: &Path) -> io::Result<Vec<PathEntry>> {
    fn visit(path: &Path, entries: &mut Vec<PathEntry>) -> io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        let is_dir = metadata.is_dir() && !metadata.file_type().is_symlink();
        if is_dir {
            for child in fs::read_dir(path)? {
                visit(&child?.path(), entries)?;
            }
        }
        entries.push(PathEntry {
            path: path.to_path_buf(),
            is_dir,
        });
        Ok(())
    }

    let mut entries = Vec::new();
    visit(path, &mut entries)?;
    Ok(entries)
}

fn read_choice(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .map_err(|error| format!("输出失败: {error}"))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|error| format!("读取输入失败: {error}"))?;
    Ok(input.trim().to_string())
}

fn interactive() -> Result<(), String> {
    let roots = Roots::detect()?;
    println!("ZCode Fresh Reset {}", env!("CARGO_PKG_VERSION"));
    {
        match fetch_update() {
            Ok(Some(update)) => {
                println!(
                    "{}",
                    tr(
                        &format!("[更新] 发现新版本 {}。", update.version),
                        &format!("[Update] Version {} is available.", update.version)
                    )
                );
                if matches!(
                    read_choice(&tr(
                        "是否立即安装更新？输入 Y 确认: ",
                        "Install now? Enter Y to confirm: "
                    ))?
                    .to_ascii_lowercase()
                    .as_str(),
                    "y" | "yes"
                ) {
                    install_update(&update)?;
                    return Ok(());
                }
            }
            Ok(None) => println!(
                "{}",
                tr(
                    "[更新] 当前已是最新版本。",
                    "[Update] You are using the latest version."
                )
            ),
            Err(error) => println!(
                "{}: {error}",
                tr("[更新] 检查失败", "[Update] Check failed")
            ),
        }
    }
    println!("[步骤 1/4] 工具已运行。");
    println!("[步骤 2/4] 正在检测 ZCode 进程和本地文件...\n");
    inspect(&roots).map_err(|error| error.to_string())?;

    if zcode_running() {
        println!("\n[警告] 检测到 ZCode 正在运行。");
        println!("强行结束 ZCode 后继续清理可能导致未保存内容丢失。");
        let choice = read_choice(&tr(
            "是否强行结束并清理？输入 Y 确认，其他键取消: ",
            "Force-close ZCode and continue? Enter Y to confirm, anything else cancels: ",
        ))?;
        if !matches!(choice.to_ascii_lowercase().as_str(), "y" | "yes") {
            println!("\n[退出] 已取消，未结束进程，未清理任何文件。");
            return Ok(());
        }
        terminate_zcode()?;
    }

    println!("\n请选择操作:");
    println!("  1 - 完整清理（自动备份）");
    println!("  2 - 安全清理（自动备份，保留登录凭据）");
    println!("  0 - 退出，不做修改");
    match read_choice("请输入 0、1 或 2，然后按回车: ")?.as_str() {
        "1" => {
            println!("\n[步骤 3/4] 开始完整清理。");
            clean(&roots, CleanOptions::default())
        }
        "2" => {
            println!("\n[步骤 3/4] 开始安全清理。");
            clean(
                &roots,
                CleanOptions {
                    safe: true,
                    ..CleanOptions::default()
                },
            )
        }
        _ => {
            println!("\n[退出] 未清理任何文件。");
            Ok(())
        }
    }
}

fn print_help(program: &OsStr) {
    let exe = Path::new(program).display();
    println!("ZCode Fresh Reset {}\n\nUsage / 用法:\n  {exe}\n  {exe} interactive\n  {exe} inspect\n  {exe} backup [--backup-dir DIR]\n  {exe} clean [--safe] [--no-backup] [--backup-dir DIR]\n  {exe} --check-update\n\nLanguage / 语言: set ZCODE_LANG=en or zh", env!("CARGO_PKG_VERSION"));
}

fn run() -> Result<(), String> {
    let mut raw = env::args_os();
    let program = raw.next().unwrap_or_else(|| "zcode-fresh-reset.exe".into());
    let args = raw
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "命令行参数不是有效文本".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match parse_args(args)? {
        Action::Gui => gui::launch()?,
        Action::InteractiveCli => interactive()?,
        Action::Help => print_help(&program),
        Action::Version => println!("zcode-fresh-reset {}", env!("CARGO_PKG_VERSION")),
        Action::CheckUpdate => check_update(),
        Action::Inspect => inspect(&Roots::detect()?).map_err(|e| e.to_string())?,
        Action::Backup(dir) => {
            let roots = Roots::detect()?;
            let root =
                dir.unwrap_or_else(|| roots.user_profile.join(".zcode").join("reset_backups"));
            backup(&roots, &root).map_err(|e| e.to_string())?;
        }
        Action::Clean(options) => clean(&Roots::detect()?, options)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    let result = match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("错误: {error}");
            ExitCode::FAILURE
        }
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_arguments_enters_gui_mode() {
        assert!(matches!(parse_args(Vec::new()).unwrap(), Action::Gui));
    }

    #[test]
    fn clean_defaults_to_protected_mode() {
        let options = CleanOptions::default();
        assert!(!options.safe);
        assert!(!options.no_backup);
    }

    #[test]
    fn parses_safe_clean() {
        match parse_args(["clean", "--safe", "--no-backup"].map(String::from)).unwrap() {
            Action::Clean(o) => {
                assert!(o.safe);
                assert!(o.no_backup);
            }
            _ => panic!("unexpected action"),
        }
    }
    #[test]
    fn resolves_candidate_path() {
        let roots = Roots {
            user_profile: PathBuf::from(r"C:\Users\tester"),
            app_data: PathBuf::from(r"C:\Users\tester\AppData\Roaming"),
        };
        assert!(roots
            .resolve(CANDIDATES[0])
            .ends_with(Path::new(".zcode").join("v2").join("credentials.json")));
    }
}

use tokio;
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::{fs::File, io::{Write, Read}, time::Instant, path::{Path, PathBuf}};
use reqwest::{self, header::CONTENT_LENGTH};
use crate::download::下載參數;

#[derive(serde::Deserialize)]
struct 附件信息 {
    #[serde(rename = "browser_download_url")]
    下載鏈接: String,
    #[serde(rename = "digest")]
    檢驗碼: String,
    #[serde(rename = "name")]
    文件名: String
}

#[derive(serde::Deserialize)]
struct 版本信息 {
    #[serde(rename = "assets")]
    附件清單: Vec<附件信息>,
}

// 獲取指定版本的全部附件下載鏈接
async fn 全部附件下載鏈接清單(版本: Option<&str>, 代理: Option<&str>, 令牌: Option<&str>) -> Vec<附件信息> {
    let 版本名 = 版本.unwrap_or("");
    let 接口鏈接 = if 版本名.is_empty() {
        "https://api.github.com/repos/rime/librime/releases/latest".to_string()
    } else {
        format!("https://api.github.com/repos/rime/librime/releases/tags/{}", &版本名)
    };
    let mut 附件下載鏈接清單: Vec<附件信息> = Vec::new();
    let 終端 = if let Some(代理地址) = 代理 {
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(代理地址).unwrap())
            .build()
            .unwrap()
    } else {
        reqwest::Client::new()
    };
    let mut 請求 = 終端
        .get(&接口鏈接)
        .header("User-Agent", "Rust reqwest");
    if let Some(token) = 令牌 {
        請求 = 請求.bearer_auth(token);
    }
    let 網絡響應 = 請求.send().await;
    match 網絡響應 {
        Ok(響應) if 響應.status().is_success() => {
            if let Ok(版本_json) = 響應.json::<版本信息>().await {
                附件下載鏈接清單.extend(版本_json.附件清單);
            } else {
                eprintln!("解析 JSON 失败");
            }
        }
        Ok(響應) => { eprintln!("API請求失敗: {}", 響應.status()); }
        Err(e) => { eprintln!("API請求錯誤: {}", e); }
    }
    附件下載鏈接清單
}

// Windows使用msvc構建的版本， macOS用universal
fn 獲取最終下載鏈接(版本: Option<&str>, 代理: Option<&str>, 令牌: Option<&str>) -> Option<附件信息> {
    let 執行期 = tokio::runtime::Runtime::new().unwrap();
    let 鏈接清單 = 執行期.block_on(全部附件下載鏈接清單(版本, 代理, 令牌));
    let 系統 = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        _ => "未支持的操作系統",
    };
    if 系統 == "未支持的操作系統" {
        eprintln!("您的系統 {} 不是Windows 或 macOS, 不支持該操作", std::env::consts::OS);
        return None;
    }
    // Windows使用msvc構建的版本， macOS用universal
    let 構建 = match 系統 {
        "Windows" => "msvc",
        "macOS" => "universal",
        _ => unreachable!(),
    };
    let 系統模式 = Regex::new(&系統).unwrap();
    let 構建模式 = Regex::new(&構建).unwrap();
    // 小狼毫根據當前系統狀態，保留正確的位數
    #[cfg(windows)]
    let 架構模式 = Regex::new(&視窗組件::獲取小狼毫架構模式()).unwrap();

    for 附件 in 鏈接清單 {
        let 鏈接 = &附件.下載鏈接;
        // 排除deps附件
        #[cfg(windows)] {
            let mut 判斷條件 = 系統模式.is_match(&鏈接) && 構建模式.is_match(&鏈接)
                && !Regex::new("deps").unwrap().is_match(&鏈接);
            判斷條件 = 判斷條件 && 架構模式.is_match(&鏈接);
            if 判斷條件 {
                return Some(附件);
            }
        }
        #[cfg(not(windows))] {
            let 判斷條件 = 系統模式.is_match(&鏈接) && 構建模式.is_match(&鏈接)
                && !Regex::new("deps").unwrap().is_match(&鏈接);
            if 判斷條件 {
                return Some(附件);
            }
        }
    }
    None
}

// 已實現 小狼毫 更新rime.dll
fn 下載並更新引擎庫(附件: &附件信息, 域名: String, 代理: Option<&str>) -> anyhow::Result<()> {
    let 路徑 = Path::new(&附件.下載鏈接);
    let mut 下載鏈接 = 附件.下載鏈接.clone();
    if 域名 != "github.com" {
        下載鏈接 = 下載鏈接.replace("github.com", &域名);
    }
    // 使用附件信息中的文件名，如果無法獲取則使用默認名稱
    let mut 文件名 = 附件.文件名.clone();
    if 文件名.is_empty() {
        文件名 = 路徑.file_name()
            .and_then(|名字| 名字.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "无效文件名".to_string());
    }
    let 終端 = if let Some(代理地址) = 代理 {
        reqwest::blocking::Client::builder()
            .proxy(reqwest::Proxy::all(代理地址).unwrap())
            .build()
            .unwrap()
    } else {
        reqwest::blocking::Client::new()
    };
    let mut 網絡響應 = 終端.get(下載鏈接.as_str()).send()?;
    if !網絡響應.status().is_success() {
        eprintln!("網絡響應不成功");
        anyhow::bail!(format!("下載文件 '{}' 失敗 orz", 網絡響應.status()));
    }
    let 目標下載文件大小 = 網絡響應
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|len| len.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // 若本地已存在且sha256匹配，直接使用
    let 本地文件路徑 = PathBuf::from(&文件名);
    if 本地文件路徑.exists() {
        match 驗證文件哈希(&本地文件路徑, &附件.檢驗碼) {
            Ok(true) => {
                println!(" '{}' 已存在且校驗碼匹配，跳過下載。", 文件名);
                return 解壓並更新引擎(&文件名);
            }
            Ok(false) => {
                println!(" '{}' 已存在但校驗碼不符，重新下載。", 文件名);
            }
            Err(e) => {
                eprintln!(" 無法校驗本地文件，將重新下載: {}", e);
            }
        }
    }
    // 創建進度條並設置樣式
    let 進度條 = ProgressBar::new(目標下載文件大小);
    進度條.set_style(
        ProgressStyle::default_bar()
        .template("{spinner}[{bar:40}] {percent}% ({bytes} / {total_bytes}) (eta: {eta}) {msg}")
        .unwrap()
        .progress_chars("█>-"),
    );
    // 創建文件
    let mut 目標文件 = match File::create(&文件名) {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow::bail!(format!("文件 {} 已經存在.", 文件名));
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                anyhow::bail!(format!("訪問權限被拒, {} 正在被使用或被鎖.", 文件名));
            } else {
                anyhow::bail!(format!("創建文件 {} 失敗: {}", 文件名, e));
            }
        },
    };
    let mut 緩存 = [0u8; 16 * 1024]; // 16KB per read
    let mut 已下載字節數 = 0u64;
    let 開始的時間 = Instant::now(); // Start the timer for speed calculation
    // 下載寫入文件，並更新進度條
    while let Ok(待讀取字節數) = 網絡響應.read(&mut 緩存) {
        // 下載已完成
        if 待讀取字節數 == 0 {
            進度條.set_message(format!("\n '{}' 下載完成!", 文件名));
            break;
        }
        目標文件.write_all(&緩存[..待讀取字節數])?;
        已下載字節數 += 待讀取字節數 as u64;
        進度條.set_position(已下載字節數);
        let 歷時 = 開始的時間.elapsed().as_secs_f64();
        let 網速 = 已下載字節數 as f64 / 歷時;
        let 網速字符串 = if 網速 >= 1_048_576.0 {
            format!("{:.2} MB/s", 網速 / 1_048_576.0)
        } else if 網速 >= 1_024.0 {
            format!("{:.2} KB/s", 網速 / 1_024.0)
        } else {
            format!("{:.2} B/s", 網速)
        };
        進度條.set_message(format!("[{}]\n {} 下載中... ", 網速字符串, 文件名));
    }
    // 結束進度條
    進度條.finish();
    println!();
    // 下載完成後驗證sha256
    if 驗證文件哈希(&本地文件路徑, &附件.檢驗碼)? {
        解壓並更新引擎(&文件名)
    } else {
        anyhow::bail!(format!("'{}' 下載後校驗碼不匹配，請重試。", 文件名))
    }
}

#[cfg(windows)]
mod 視窗組件 {
    use windows::Win32::System::SystemInformation::{
        GetNativeSystemInfo, PROCESSOR_ARCHITECTURE, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, SYSTEM_INFO
    };
    use windows_version::OsVersion;
    use winreg::RegKey;
    use std::ffi::OsStr;

    fn 檢查架構(arch: PROCESSOR_ARCHITECTURE) -> bool {
        let mut info = SYSTEM_INFO::default();
        unsafe {
            GetNativeSystemInfo(&mut info);
            info.Anonymous.Anonymous.wProcessorArchitecture == arch
        }
    }

    fn 系統是amd64架構() -> bool { 檢查架構(PROCESSOR_ARCHITECTURE_AMD64) }

    fn 系統是arm64架構() -> bool { 檢查架構(PROCESSOR_ARCHITECTURE_ARM64) }

    fn 版本高於_win11() -> bool {
        let 系統版本 = OsVersion::current();
        系統版本.major > 10 && 系統版本.build >= 22000
    }

    pub fn 獲取小狼毫架構模式() -> String {
        if 版本高於_win11() {
            if 系統是arm64架構() || 系統是amd64架構() {"x64".to_string()}
            else { "x86".to_string() }
        } else {
            if 系統是amd64架構() {"x64".to_string()}
            else { "x86".to_string() }
        }
    }

    pub fn 獲取小狼毫程序目錄() -> Option<String> {
        let 註冊表路徑 = {
            if 系統是arm64架構() || 系統是amd64架構() { OsStr::new("SOFTWARE\\WOW6432Node\\Rime\\Weasel") }
            else { OsStr::new("SOFTWARE\\Rime\\Weasel") }
        };
        RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
            .open_subkey(註冊表路徑)
            .and_then(|注冊表鍵| 注冊表鍵.get_value("WeaselRoot"))
            .ok()
    }

}

#[cfg(windows)]
fn 解壓並更新引擎(文件名: &String) -> anyhow::Result<()>{
    match 視窗組件::獲取小狼毫程序目錄() {
        Some(小狼毫根目錄) => {
            let 小狼毫算法服務 = Path::new(&小狼毫根目錄).join("WeaselServer.exe");
            if 小狼毫算法服務.exists() {
                let 目錄名 = 文件名.replace(".7z", "");
                // 解壓壓縮包
                sevenz_rust2::decompress_file(&文件名, &目錄名).expect("完成!");
                println!(" '{}' 已解壓到 '{}'", &文件名, &目錄名);
                // 退出小狼毫算法服務
                let _ = std::process::Command::new(&小狼毫算法服務)
                    .arg("/q")
                    .spawn()?
                    .wait()?;
                // 等待500毫秒, 待小狼毫算法服務退出完成
                std::thread::sleep(std::time::Duration::from_millis(500));
                println!(" 小狼毫服務 '{}' 已退出", &小狼毫算法服務.display());
                let 目標庫文件 = Path::new(&小狼毫根目錄).join("rime.dll");
                // 複製新文件
                match std::fs::copy(Path::new(&目錄名.as_str()).join("dist/lib/rime.dll"), &目標庫文件) {
                    Ok(_) => { println!(" 中州韻引擎 '{}' 已更新", &目標庫文件.display()) },
                    Err(e) => { eprintln!("複製文件時發生錯誤: {}", e) }
                }
                // 啓動小狼毫算法服務
                let _ = std::process::Command::new(&小狼毫算法服務)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()?;
                println!(" 小狼毫服務 '{}' 已重啓", &小狼毫算法服務.display());
                // 刪除臨時目錄
                match std::fs::remove_dir_all(&目錄名) {
                    Ok(_) => { println!(" 臨時目錄 '{}' 已刪除", &目錄名); },
                    Err(e) => { anyhow::bail!(" 無法刪除目錄 '{}', {}", &目錄名, e); },
                }
                Ok(())
            } else {
                anyhow::bail!("WeaselServer.exe 未找到.");
            }
        },
        None => { anyhow::bail!("WeaselRoot 未成功讀取."); }
    }
}

#[cfg(not(windows))]
fn 解壓並更新引擎(_文件名: &String) -> anyhow::Result<()>{
    // 解壓更新鼠須管的rime引擎庫
    todo!("還不會做呢！");
}

pub fn 更新引擎庫(版本: &String, 參數: &下載參數) -> anyhow::Result<()>  {
    let 代理 = 參數.代理地址();
    // token 允許從參數傳入，也允許回退到環境變量；
    let token_owned = match 參數.令牌() {
        Some(t) => Some(t.to_string()),
        None => {
            std::env::var("GITHUB_TOKEN").ok().filter(|s| !s.is_empty())
                .or_else(|| std::env::var("GH_TOKEN").ok().filter(|s| !s.is_empty()))
        }
    };
    let 令牌 = token_owned.as_deref();
    let 附件 = 獲取最終下載鏈接(Some(版本), 代理, 令牌);
    let 倉庫 = 參數.倉庫域名().unwrap_or_else(|| "github.com").to_string();
    if let Some(附件) = 附件 {
        下載並更新引擎庫(&附件, 倉庫, 代理)
    } else {
        anyhow::bail!("未找到合適的下載鏈接.");
    }
}

fn 文件_sha256(path: &Path) -> anyhow::Result<String> {
    let mut 文件 = File::open(path)?;
    let mut 雜湊器 = Sha256::new();
    let mut 緩衝 = [0u8; 16 * 1024];
    loop {
        let 已讀 = 文件.read(&mut 緩衝)?;
        if 已讀 == 0 { break; }
        雜湊器.update(&緩衝[..已讀]);
    }
    Ok(format!("{:x}", 雜湊器.finalize()))
}

fn 驗證文件哈希(path: &Path, 摘要: &str) -> anyhow::Result<bool> {
    let 預期 = 摘要.strip_prefix("sha256:").unwrap_or(摘要).to_lowercase();
    let 實際 = 文件_sha256(path)?.to_lowercase();
    Ok(實際 == 預期)
}

use tokio;
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle};
use std::{fs::{metadata, File}, io::{Write, Read}, time::Instant, path::Path};
use reqwest::{blocking::get, self, header::CONTENT_LENGTH};
use crate::download::下載參數;

#[derive(serde::Deserialize)]
struct Asset {
    browser_download_url: String,
}

#[derive(serde::Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

// 獲取指定版本的全部附件下載鏈接
async fn 全部附件下載鏈接清單(版本: Option<&str>) -> Vec<String> {
    let 版本名 = 版本.unwrap_or("");
    let 接口鏈接 = if 版本名.is_empty() {
        "https://api.github.com/repos/rime/librime/releases/latest".to_string()
    } else {
        format!("https://api.github.com/repos/rime/librime/releases/tags/{}", &版本名)
    };
    let mut 附件下載鏈接清單: Vec<String> = Vec::new();
    let 網絡響應 = reqwest::Client::new()
        .get(&接口鏈接)
        .header("User-Agent", "Rust reqwest")
        .send()
        .await;
    match 網絡響應 {
        Ok(響應) if 響應.status().is_success() => {
            if let Ok(release) = 響應.json::<Release>().await {
                for asset in release.assets {
                    附件下載鏈接清單.push(asset.browser_download_url);
                }
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
fn 獲取最終下載鏈接(版本: Option<&str>) -> Option<String> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let 鏈接清單 = runtime.block_on(全部附件下載鏈接清單(版本));
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
    let 架構模式 = Regex::new(&winutils::獲取小狼毫架構模式()).unwrap();

    for 鏈接 in 鏈接清單 {
        // 排除deps附件
        let 判斷條件 = 系統模式.is_match(&鏈接) && 構建模式.is_match(&鏈接) 
            && !Regex::new("deps").unwrap().is_match(&鏈接);
        #[cfg(windows)] {
            判斷條件 = 判斷條件 && 架構模式.is_match(&鏈接);
        }
        if 判斷條件 {
            return Some(鏈接);
        }
    }
    None
}

// 已實現 小狼毫 更新rime.dll
fn 下載並更新引擎庫(鏈接: &String, host: String) -> anyhow::Result<()>{
    let 路徑 = Path::new(&鏈接);
    let mut 下載鏈接 = 鏈接.clone();
    if !host.is_empty() {
        下載鏈接 = 下載鏈接.replace("github.com", &host);
    }
    let 文件名 = 路徑.file_name()
        .and_then(|名字| 名字.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "无效文件名".to_string());
    let mut 網絡響應 = get(下載鏈接.as_str())?;
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
    // 檢查要下載的文件是不是已經存在並且大小和計劃下載大小一樣，已存則跳過下載
    if let Ok(元數據) = metadata(&文件名) {
        let 現存文件大小 = 元數據.len();
        if 現存文件大小 == 目標下載文件大小 {
            println!(" '{}' 已存在並且已是最新.", 文件名);
            return 解壓並更新引擎(&文件名);
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
    解壓並更新引擎(&文件名)
}

#[cfg(windows)]
mod winutils {
    use windows::Win32::System::SystemInformation::{
        GetNativeSystemInfo, PROCESSOR_ARCHITECTURE, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, SYSTEM_INFO
    };
    use windows_version::OsVersion;
    use winreg::RegKey;
    use std::ffi::OsStr;

    fn is_arch(arch: PROCESSOR_ARCHITECTURE) -> bool {
        let mut info = SYSTEM_INFO::default();
        unsafe {
            GetNativeSystemInfo(&mut info);
            info.Anonymous.Anonymous.wProcessorArchitecture == arch
        }
    }

    fn is_native_amd64() -> bool { is_arch(PROCESSOR_ARCHITECTURE_AMD64) }

    fn is_native_arm64() -> bool { is_arch(PROCESSOR_ARCHITECTURE_ARM64) }

    fn is_at_least_win11() -> bool {
        let ver = OsVersion::current();
        ver.major > 10 && ver.build >= 22000
    }

    pub fn 獲取小狼毫架構模式() -> String {
        if is_at_least_win11() {
            if is_native_arm64() || is_native_amd64() {"x64".to_string()}
            else { "x86".to_string() }
        } else {
            if is_native_amd64() {"x64".to_string()}
            else { "x86".to_string() }
        }
    }

    pub fn 獲取小狼毫程序目錄() -> Option<String> {
        let 註冊表路徑 = {
            if is_native_arm64() || is_native_amd64() { OsStr::new("SOFTWARE\\WOW6432Node\\Rime\\Weasel") }
            else { OsStr::new("SOFTWARE\\Rime\\Weasel") }
        };
        RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
            .open_subkey(註冊表路徑)
            .and_then(|key| key.get_value("WeaselRoot"))
            .ok()
    }

}

#[cfg(windows)]
fn 解壓並更新引擎(文件名: &String) -> anyhow::Result<()>{
    match winutils::獲取小狼毫程序目錄() {
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
    todo!("還不會做呢！");
}

pub fn 更新引擎庫(版本: &String, 參數: &下載參數) -> anyhow::Result<()>  {
    參數.設置代理();
    let 鏈接 = 獲取最終下載鏈接(Some(版本));
    if let Some(鏈接) = 鏈接 {
        下載並更新引擎庫(&鏈接, 參數.host.clone().unwrap_or("".to_string()))
    } else {
        anyhow::bail!("未找到合適的下載鏈接.");
    }
}

use std::path::PathBuf;
use structopt::StructOpt;

mod deploy;
mod download;
mod get_rime;
mod install;
mod package;
mod recipe;
mod rime_levers;

use deploy::部署輸入法固件;
use download::{下載參數, 下載配方包};
use install::安裝配方;
use recipe::配方名片;
use rime_levers::{
    加入輸入方案列表, 測試輸入法固件, 製備輸入法固件, 設置引擎構建與整備參數, 選擇輸入方案,
    配置補丁,
};

#[derive(Debug, StructOpt)]
pub struct 場地參數 {
    /// 配方包下載緩存目錄
    #[structopt(
        long = "download-dir",
        env = "DOWNLOAD_DIR",
        default_value = "pkg",
        value_name = "DIR",
        global = true
    )]
    pub 下載場地: PathBuf,
    /// 編譯期源文件目錄
    #[structopt(
        long = "source-dir",
        env = "SOURCE_DIR",
        default_value = "src",
        value_name = "DIR",
        global = true
    )]
    pub 源碼場地: PathBuf,
    /// 運行時產物整備目錄
    #[structopt(
        long = "staging-dir",
        env = "STAGING_DIR",
        default_value = "stage",
        value_name = "DIR",
        global = true
    )]
    pub 整備場地: PathBuf,
    /// 部署目標目錄
    #[structopt(
        long = "target-dir",
        env = "TARGET_DIR",
        default_value = "target",
        value_name = "DIR",
        global = true
    )]
    pub 目標場地: PathBuf,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "rime", about = "Rime 配方管理器")]
struct 命令行參數 {
    #[structopt(flatten)]
    場地: 場地參數,
    #[structopt(subcommand)]
    命令: 子命令,
}

#[derive(Debug, StructOpt)]
enum 子命令 {
    /// 加入輸入方案列表
    Add {
        /// 要向列表中追加的輸入方案
        schemata: Vec<String>,
    },
    /// 構建輸入法固件
    Build,
    /// 部署輸入法固件到目標位置
    Deploy,
    /// 下載配方包
    Download {
        /// 要下載的配方包
        recipes: Vec<String>,
        #[structopt(flatten)]
        下載參數: 下載參數,
    },
    /// 更新引擎庫
    Get {
        tag: Option<String>,
        #[structopt(flatten)]
        下載參數: 下載參數,
    },
    /// 安裝配方
    Install {
        /// 跳過下載步驟，直接從本地緩存倉庫安裝
        #[structopt(long = "no-download")]
        不下載: bool,
        /// 要安裝的配方
        recipes: Vec<String>,
        #[structopt(flatten)]
        下載參數: 下載參數,
    },
    /// 新建配方
    New {
        /// 配方名字
        _name: Option<String>,
    },
    /// 配置補丁
    Patch {
        /// 目標配置
        config: String,
        /// 紐
        key: String,
        /// 值
        value: String,
    },
    /// 選擇輸入方案
    Select {
        /// 選中的輸入方案
        schema: String,
    },
    /// 在整備區測試構建好的輸入法數據
    Test {
        /// 要測試的輸入方案（留空則測試全部已構建方案）
        schemata: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let 參數 = 命令行參數::from_args();
    log::debug!("參數: {:?}", 參數);
    let 場地 = &參數.場地;

    match 參數.命令 {
        子命令::Add { schemata } => {
            設置引擎構建與整備參數(&場地.源碼場地, &場地.整備場地)?;
            加入輸入方案列表(&schemata)?;
        }
        子命令::Build => {
            設置引擎構建與整備參數(&場地.源碼場地, &場地.整備場地)?;
            製備輸入法固件()?;
        }
        子命令::Deploy => {
            部署輸入法固件(&場地.整備場地, &場地.目標場地)?;
        }
        子命令::Download {
            recipes, 下載參數
        } => {
            let 衆配方 = recipes
                .iter()
                .map(|rx| 配方名片::from(rx.as_str()))
                .collect::<Vec<_>>();
            下載配方包(&衆配方, &場地.下載場地, &下載參數)?;
        }
        子命令::Get { tag, 下載參數 } => {
            let 目標版本 = tag.unwrap_or_default();
            get_rime::更新引擎庫(&目標版本, &下載參數)?;
        }
        子命令::Install {
            不下載,
            recipes,
            下載參數,
        } => {
            let 衆配方 = recipes
                .iter()
                .map(|rx| 配方名片::from(rx.as_str()))
                .collect::<Vec<_>>();
            if !不下載 {
                下載配方包(&衆配方, &場地.下載場地, &下載參數)?;
            }
            for 配方 in &衆配方 {
                安裝配方(配方, &場地.下載場地, &場地.源碼場地, &場地.整備場地)?;
            }
        }
        子命令::Patch { config, key, value } => {
            設置引擎構建與整備參數(&場地.源碼場地, &場地.整備場地)?;
            配置補丁(&config, &key, &value)?;
        }
        子命令::Select { schema } => {
            設置引擎構建與整備參數(&場地.源碼場地, &場地.整備場地)?;
            選擇輸入方案(&schema)?;
        }
        子命令::Test { schemata } => {
            測試輸入法固件(&場地.整備場地, &schemata)?;
        }
        _ => todo!("還沒做呢"),
    }

    Ok(())
}

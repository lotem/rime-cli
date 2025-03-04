use std::path::PathBuf;
use structopt::StructOpt;

mod download;
mod install;
mod package;
mod recipe;
mod rime_levers;

use download::下載配方包;
use install::安裝配方;
use package::配方包;
use recipe::配方名片;
use rime_levers::{
    加入輸入方案列表, 製備輸入法固件, 設置引擎啓動參數, 選擇輸入方案, 配置補丁
};

#[derive(Debug, StructOpt)]
#[structopt(about = "Rime 配方管理器")]
struct 命令行參數 {
    // 代理 URL 作爲選項
    #[structopt(short,long = "proxy", global = true, help = "代理服務器地址")]
    proxy: Option<String>,
    #[structopt(short,long = "host", global = true, help = "倉庫域名")]
    host: Option<String>,
    // 子命令
    #[structopt(subcommand)]
    子命令: 子命令,
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
    },
    /// 安裝配方
    Install {
        /// 要安裝的配方
        recipes: Vec<String>,
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
}

fn 設置全局參數(host: &String, proxy: &String) {
    if !host.is_empty() {
        std::env::set_var("repo_host", host);
        log::debug!("設置倉庫域名 {}", host);
    }
    if !proxy.is_empty() {
        std::env::set_var("http_proxy", proxy);
        std::env::set_var("https_proxy", proxy);
        log::debug!("設置代理 {}", proxy);
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let 命令行參數 = 命令行參數::from_args();
    log::debug!("參數: {:?}", 命令行參數);

    let 代理地址 = 命令行參數.proxy.unwrap_or("".to_string());
    let 倉庫域名 = 命令行參數.host.unwrap_or("".to_string());
    match 命令行參數.子命令 {
        子命令::Add { schemata } => {
            let 還不知道怎麼傳過來 = PathBuf::from(".");
            設置引擎啓動參數(&還不知道怎麼傳過來)?;
            加入輸入方案列表(&schemata)?;
        }
        子命令::Build => {
            let 還不知道怎麼傳過來 = PathBuf::from(".");
            設置引擎啓動參數(&還不知道怎麼傳過來)?;
            製備輸入法固件()?;
        }
        子命令::Download { recipes } => {
            設置全局參數(&倉庫域名, &代理地址);
            下載配方包(
                recipes
                    .iter()
                    .map(|rx| 配方包::from(rx.as_str()))
                    .collect::<Vec<_>>(),
            )?;
        }
        子命令::Install { recipes } => {
            設置全局參數(&倉庫域名, &代理地址);
            for rx in recipes {
                安裝配方(配方名片::from(rx.as_str()));
            }
        }
        子命令::Patch { config, key, value } => {
            let 還不知道怎麼傳過來 = PathBuf::from(".");
            設置引擎啓動參數(&還不知道怎麼傳過來)?;
            配置補丁(&config, &key, &value)?;
        }
        子命令::Select { schema } => {
            選擇輸入方案(&schema)?;
        }
        _ => todo!("還沒做呢"),
    }

    Ok(())
}

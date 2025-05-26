use std::path::PathBuf;
use structopt::StructOpt;

mod download;
mod install;
mod package;
mod recipe;
mod rime_levers;
mod get_rime;

use download::{下載參數, 下載配方包};
use install::安裝配方;
use recipe::配方名片;
use rime_levers::{
    加入輸入方案列表, 製備輸入法固件, 設置引擎啓動參數, 選擇輸入方案, 配置補丁
};

#[derive(Debug, StructOpt)]
#[structopt(about = "Rime 配方管理器")]
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
    /// 安裝配方
    Install {
        /// 要安裝的配方
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

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let 命令行參數 = 子命令::from_args();
    log::debug!("參數: {:?}", 命令行參數);

    match 命令行參數 {
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
        子命令::Download {
            recipes, 下載參數
        } => {
            let 衆配方 = recipes
                .iter()
                .map(|rx| 配方名片::from(rx.as_str()))
                .collect::<Vec<_>>();
            下載配方包(&衆配方, 下載參數)?;
        }
        子命令::Install {
            recipes, 下載參數
        } => {
            let 衆配方 = recipes
                .iter()
                .map(|rx| 配方名片::from(rx.as_str()))
                .collect::<Vec<_>>();
            下載配方包(&衆配方, 下載參數)?;
            for 配方 in &衆配方 {
                安裝配方(配方)?;
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
        子命令::Get { tag, 下載參數 } => {
            let 目標版本 = tag.unwrap_or("".to_string());
            get_rime::更新引擎庫(&目標版本, &下載參數)?;
        }
        _ => todo!("還沒做呢"),
    }

    Ok(())
}

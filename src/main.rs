use structopt::StructOpt;

mod recipe;
mod package;
mod download;
mod install;

use recipe::配方名片;
use package::配方包;
use download::下載配方包;
use install::安裝配方;

#[derive(Debug, StructOpt)]
#[structopt(about = "Rime 配方管理器")]
enum 子命令 {
    /// 下載配方包
    Download {
        recipes: Vec<String>,
    },
    /// 安裝配方
    Install {
        recipes: Vec<String>,
    },
    /// 配置補丁
    Patch {
        /// 目標配置文件
        config: String,
        /// 紐
        key: String,
        /// 值
        value: String,
    },
    /// 新建配方
    NewRecipe {
        /// 配方名字
        name: Option<String>,
    },
    /// 構建輸入法固件
    Build,
    /// 部署輸入法固件到目標位置
    Deploy,
}

fn main() {
    env_logger::init();

    let 命令行參數 = 子命令::from_args();
    log::debug!("參數: {:?}", 命令行參數);
    match 命令行參數 {
        子命令::Download { ref recipes } => {
            for rx in recipes {
                下載配方包(配方包::from(rx.as_str()).倉庫);
            }
        },
        子命令::Install { ref recipes } => {
            for rx in recipes {
                安裝配方(配方名片::from(rx.as_str()));
            }
        },
        _ => todo!("還沒做呢"),
    }
}

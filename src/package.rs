use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use crate::recipe::配方名片;

#[derive(Clone)]
pub struct 代碼庫地址 {
    pub 網址: String,
    pub 分支: Option<String>,
}

impl fmt::Display for 代碼庫地址 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.分支 {
            Some(分支) => write!(f, "{}@{}", self.網址, 分支),
            None => write!(f, "{}", self.網址),
        }
    }
}

#[derive(Clone)]
pub struct 配方包 {
    pub 配方: 配方名片,
    pub 倉庫: 代碼庫地址,
    // pub 內容文件 Vec<PathBuf>,
}

impl 配方包 {
    pub fn 本地路徑(&self) -> PathBuf {
        ["pkg", self.配方.方家.as_str(), self.配方.名字.as_str()]
            .iter()
            .collect()
    }

    pub fn 按倉庫分組(衆配方包: Vec<配方包>) -> HashMap<配方名片, Vec<配方包>> {
        let mut 按倉庫分組 = HashMap::new();
        衆配方包.into_iter().for_each(|包| {
            let 包名 = 配方名片 {
                版本: None,
                ..包.配方.clone()
            };
            按倉庫分組.entry(包名).or_insert_with(Vec::new).push(包);
        });
        按倉庫分組
    }
}

impl From<&str> for 配方包 {
    fn from(source: &str) -> Self {
        let 配方 = 配方名片::from(source);
        Self::from(配方)
    }
}

impl From<配方名片> for 配方包 {
    fn from(source: 配方名片) -> Self {
        let 倉庫 = 配方倉庫地址(&source);
        Self {
            配方: source, 倉庫
        }
    }
}

fn 配方倉庫地址(配方: &配方名片) -> 代碼庫地址 {
    let 域名 = std::env::var("repo_host").unwrap_or("github.com".to_string());
    代碼庫地址 {
        網址: format!("https://{}/{}/{}.git", 域名, 配方.方家, 配方.名字),
        分支: 配方.版本.clone(),
    }
}

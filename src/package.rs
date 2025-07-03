use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use crate::recipe::配方名片;

#[derive(Clone)]
pub struct 配方包<'a> {
    pub 配方: 配方名片,
    pub 倉庫域名: Option<&'a str>,
    // pub 內容文件 Vec<PathBuf>,
}

impl 配方包<'_> {
    pub fn 倉庫地址(&self) -> String {
        format!(
            "https://{}/{}/{}.git",
            self.倉庫域名.unwrap_or("github.com"),
            self.配方.方家,
            self.配方.名字
        )
    }

    pub fn 倉庫分支(&self) -> Option<&str> {
        self.配方.版本.as_deref()
    }

    pub fn 本地路徑(&self) -> PathBuf {
        ["pkg", self.配方.方家.as_str(), self.配方.名字.as_str()]
            .iter()
            .collect()
    }

    pub fn 按倉庫分組<'a>(
        衆配方: &[配方名片],
        倉庫域名: Option<&'a str>,
    ) -> HashMap<配方名片, Vec<配方包<'a>>> {
        let mut 按倉庫分組 = HashMap::new();
        衆配方.iter().for_each(|配方| {
            let 包名 = 配方名片 {
                版本: None,
                ..配方.clone()
            };
            按倉庫分組
                .entry(包名)
                .or_insert_with(Vec::new)
                .push(配方包 {
                    配方: 配方.clone(),
                    倉庫域名,
                });
        });
        按倉庫分組
    }
}

impl fmt::Display for 配方包<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.倉庫分支() {
            Some(分支) => write!(f, "{}@{}", self.倉庫地址(), 分支),
            None => write!(f, "{}", self.倉庫地址()),
        }
    }
}

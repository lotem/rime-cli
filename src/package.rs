use std::fmt;

use crate::recipe::配方名片;

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

pub struct 配方包 {
    pub 配方: 配方名片,
    pub 倉庫: 代碼庫地址,
    // pub 內容文件 Vec<PathBuf>,
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
        Self { 配方: source, 倉庫 }
    }
}

fn 配方倉庫地址(配方: &配方名片) -> 代碼庫地址 {
    代碼庫地址 {
        網址: format!("https://github.com/{}/{}", 配方.方家, 配方.名字),
        分支: 配方.版本.clone(),
    }
}

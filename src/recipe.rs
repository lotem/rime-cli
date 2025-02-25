use std::fmt;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct 配方名片 {
    pub 方家: String,
    pub 名字: String,
    pub 版本: Option<String>,
}

impl fmt::Display for 配方名片 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.方家, self.名字)?;
        if let Some(版本) = &self.版本 {
            write!(f, "@{}", 版本)?;
        }
        Ok(())
    }
}

impl From<&str> for 配方名片 {
    fn from(source: &str) -> Self {
        // 有冇版本?
        let (全名, 版本) = source
            .split_once('@')
            .map(|(全名, 版本)| (全名, Some(版本.to_owned())))
            .unwrap_or((source, None));
        // 哪位方家?
        let (方家, 名字) = 全名
            .split_once('/')
            .map(|(方家, 名字)| (方家.to_owned(), 名字.to_owned()))
            .unwrap_or(("rime".to_owned(), 規範的配方名字(全名)));
        Self {
            方家, 名字, 版本
        }
    }
}

fn 規範的配方名字(名字: &str) -> String {
    // 規範規範, 要包含 rime 數據倉庫前綴
    if 名字.starts_with("rime-") {
        名字.to_owned()
    } else {
        format!("rime-{名字}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 測試配方名片_姓名全不帶版本() {
        let 配方 = 配方名片::from("lotem/rime-zhengma");
        assert_eq!(配方.方家, "lotem");
        assert_eq!(配方.名字, "rime-zhengma");
        assert_eq!(配方.版本, None);
    }

    #[test]
    fn 測試配方名片_姓名全帶版本() {
        let 配方 = 配方名片::from("lotem/rime-octagram-data@hant");
        assert_eq!(配方.方家, "lotem");
        assert_eq!(配方.名字, "rime-octagram-data");
        assert_eq!(配方.版本, Some("hant".to_owned()));
    }

    #[test]
    fn 測試配方名片_只有名字() {
        let 配方 = 配方名片::from("luna-pinyin");
        assert_eq!(配方.方家, "rime");
        assert_eq!(配方.名字, "rime-luna-pinyin");
        assert_eq!(配方.版本, None);
    }

    #[test]
    fn 測試配方名片_規範的名字() {
        let 配方 = 配方名片::from("rime-luna-pinyin");
        assert_eq!(配方.方家, "rime");
        assert_eq!(配方.名字, "rime-luna-pinyin");
        assert_eq!(配方.版本, None);
    }

    #[test]
    fn 測試配方名片_只有名字和版本() {
        let 配方 = 配方名片::from("bopomofo@master");
        assert_eq!(配方.方家, "rime");
        assert_eq!(配方.名字, "rime-bopomofo");
        assert_eq!(配方.版本, Some("master".to_owned()));
    }
}

use crate::package::配方包;

use git2::Repository;
use std::path::PathBuf;

pub fn 下載配方包(包: 配方包) {
    log::debug!("下載配方包: {}, 位於 {}", 包.配方, 包.倉庫);
    todo!("還沒做呢")
}

fn 搬運倉庫(包: &配方包, 本地路徑: &PathBuf) -> anyhow::Result<()> {
    let 網址 = &包.倉庫.網址;
    let 倉庫 = Repository::clone(網址, 本地路徑)?;
    Ok(())
}

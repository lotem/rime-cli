use std::path::Path;

use anyhow::bail;

use crate::install::遞歸複製目錄;

/// 將 STAGING_DIR (stage/) 中的可重現運行時資源與 build/ 產物部署至 TARGET_DIR
pub fn 部署輸入法固件(整備場地: &Path, 目標場地: &Path) -> anyhow::Result<()> {
    log::debug!(
        "部署輸入法固件: {} -> {}",
        整備場地.display(),
        目標場地.display()
    );

    if !整備場地.exists() {
        bail!(
            "找不到整備區目錄: {}（請先執行 rime build）",
            整備場地.display()
        );
    }

    遞歸複製目錄(整備場地, 目標場地)?;
    log::info!("已部署至目標位置: {}", 目標場地.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use std::fs;

    #[test]
    fn 測試部署輸入法固件() {
        let 臨時場地 = tempfile::tempdir().unwrap();
        let 整備場地 = 臨時場地.path().join("stage");
        let 目標場地 = 臨時場地.path().join("target");

        fs::create_dir_all(整備場地.join("build")).unwrap();
        fs::create_dir_all(整備場地.join("opencc")).unwrap();
        fs::write(整備場地.join("build/default.yaml"), "schema_list: []\n").unwrap();
        fs::write(整備場地.join("build/luna_pinyin.table.bin"), b"BIN").unwrap();
        fs::write(整備場地.join("opencc/t2s.json"), "{}").unwrap();
        fs::write(整備場地.join("zh-hant-t-essay-bgw.gram"), b"GRAM").unwrap();

        assert_ok!(部署輸入法固件(&整備場地, &目標場地));

        assert!(目標場地.join("build/default.yaml").exists());
        assert!(目標場地.join("build/luna_pinyin.table.bin").exists());
        assert!(目標場地.join("opencc/t2s.json").exists());
        assert!(目標場地.join("zh-hant-t-essay-bgw.gram").exists());
    }
}

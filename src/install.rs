use std::fs;
use std::path::Path;

use anyhow::{bail, Context};

use crate::download::檢出倉庫分支;
use crate::package::配方包;
use crate::recipe::配方名片;

/// 安裝單個目標配方（將編譯期源碼複製至 SOURCE_DIR，將運行時資源複製至 STAGING_DIR）
pub fn 安裝配方(
    配方: &配方名片,
    下載場地: &Path,
    源碼場地: &Path,
    整備場地: &Path,
) -> anyhow::Result<()> {
    log::debug!(
        "安裝配方: {配方}, 下載場地: {}, 源碼場地: {}, 整備場地: {}",
        下載場地.display(),
        源碼場地.display(),
        整備場地.display()
    );

    let 包 = 配方包 {
        配方: 配方.clone(),
        倉庫域名: None,
    };
    let 本地倉庫 = 包.本地路徑(下載場地);
    if !本地倉庫.exists() {
        bail!(
            "找不到配方包本地倉庫: {}（請先執行 rime download 或移除 --no-download）",
            本地倉庫.display()
        );
    }

    // 若本地倉庫爲 Git 倉庫，確保檢出配方所指定的分支
    檢出倉庫分支(&本地倉庫, 包.倉庫分支())?;

    for 條目結果 in fs::read_dir(&本地倉庫)
        .with_context(|| format!("無法讀取配方包目錄: {}", 本地倉庫.display()))?
    {
        let 條目 = 條目結果?;
        let 路徑 = 條目.path();
        let 文件名 = match 條目.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        // 忽略隱藏文件與版本控制目錄（如 .git, .github, .gitignore）
        if 文件名.starts_with('.') {
            continue;
        }

        let 文件類型 = 條目.file_type()?;
        if 文件類型.is_dir() {
            if 是運行時資源目錄(&文件名) {
                let 目標子目錄 = 整備場地.join(&文件名);
                遞歸複製目錄(&路徑, &目標子目錄)?;
            }
        } else if 文件類型.is_file() {
            if 是編譯期源文件(&文件名) {
                fs::create_dir_all(源碼場地)?;
                let 目標文件 = 源碼場地.join(&文件名);
                log::info!("安裝源文件: {} -> {}", 路徑.display(), 目標文件.display());
                fs::copy(&路徑, &目標文件)?;
            } else if 是運行時資源文件(&文件名) {
                fs::create_dir_all(整備場地)?;
                let 目標文件 = 整備場地.join(&文件名);
                log::info!(
                    "安裝運行時資源: {} -> {}",
                    路徑.display(),
                    目標文件.display()
                );
                fs::copy(&路徑, &目標文件)?;
            }
        }
    }

    Ok(())
}

fn 是編譯期源文件(文件名: &str) -> bool {
    if 文件名.ends_with(".yaml") {
        // 仿照 rime/plum：排除 *.custom.yaml 與 *.recipe.yaml
        return !文件名.ends_with(".custom.yaml") && !文件名.ends_with(".recipe.yaml");
    }
    if 文件名.ends_with(".txt") {
        let 大寫名 = 文件名.to_ascii_uppercase();
        let 是說明文檔 = 大寫名.starts_with("README")
            || 大寫名.starts_with("LICENSE")
            || 大寫名.starts_with("COPYING")
            || 大寫名.starts_with("AUTHORS")
            || 大寫名.starts_with("CHANGELOG")
            || 文件名.eq_ignore_ascii_case("CMakeLists.txt");
        return !是說明文檔;
    }
    false
}

fn 是運行時資源文件(文件名: &str) -> bool {
    文件名.ends_with(".gram") || 文件名 == "rime.lua"
}

fn 是運行時資源目錄(目錄名: &str) -> bool {
    目錄名 == "opencc" || 目錄名 == "lua"
}

pub(crate) fn 遞歸複製目錄(源目錄: &Path, 目標目錄: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(目標目錄)?;
    for 條目結果 in fs::read_dir(源目錄)? {
        let 條目 = 條目結果?;
        let 源路徑 = 條目.path();
        let 目標路徑 = 目標目錄.join(條目.file_name());
        let 文件類型 = 條目.file_type()?;
        if 文件類型.is_dir() {
            遞歸複製目錄(&源路徑, &目標路徑)?;
        } else if 文件類型.is_file() {
            fs::copy(&源路徑, &目標路徑)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;

    #[test]
    fn 測試安裝配方_源碼與運行時資源分流() {
        let 臨時場地 = tempfile::tempdir().unwrap();
        let 下載場地 = 臨時場地.path().join("pkg");
        let 源碼場地 = 臨時場地.path().join("src");
        let 整備場地 = 臨時場地.path().join("stage");

        let 倉庫目錄 = 下載場地.join("rime").join("rime-luna-pinyin");
        fs::create_dir_all(倉庫目錄.join("opencc")).unwrap();
        fs::create_dir_all(倉庫目錄.join("lua")).unwrap();
        fs::create_dir_all(倉庫目錄.join(".git")).unwrap();

        // 編譯期源文件
        fs::write(
            倉庫目錄.join("luna_pinyin.schema.yaml"),
            "schema:\n  schema_id: luna_pinyin\n",
        )
        .unwrap();
        fs::write(
            倉庫目錄.join("luna_pinyin.dict.yaml"),
            "---\nname: luna_pinyin\n...\n",
        )
        .unwrap();
        fs::write(倉庫目錄.join("essay.txt"), "測試\tce shi\t100\n").unwrap();

        // 應被忽略的文件
        fs::write(倉庫目錄.join("luna_pinyin.custom.yaml"), "patch: {}\n").unwrap();
        fs::write(倉庫目錄.join("luna_pinyin.recipe.yaml"), "recipe: {}\n").unwrap();
        fs::write(倉庫目錄.join("README.md"), "# readme\n").unwrap();
        fs::write(倉庫目錄.join("LICENSE.txt"), "MIT\n").unwrap();
        fs::write(倉庫目錄.join(".git/config"), "").unwrap();

        // 運行時資源文件
        fs::write(倉庫目錄.join("zh-hant-t-essay-bgw.gram"), b"GRAM").unwrap();
        fs::write(倉庫目錄.join("rime.lua"), "-- lua entry\n").unwrap();
        fs::write(倉庫目錄.join("opencc/t2s.json"), "{}").unwrap();
        fs::write(倉庫目錄.join("lua/filter.lua"), "return {}").unwrap();

        let 配方 = 配方名片::from("luna-pinyin");
        assert_ok!(安裝配方(&配方, &下載場地, &源碼場地, &整備場地));

        // 驗證 SOURCE_DIR (src/)
        assert!(源碼場地.join("luna_pinyin.schema.yaml").exists());
        assert!(源碼場地.join("luna_pinyin.dict.yaml").exists());
        assert!(源碼場地.join("essay.txt").exists());
        assert!(!源碼場地.join("luna_pinyin.custom.yaml").exists());
        assert!(!源碼場地.join("luna_pinyin.recipe.yaml").exists());
        assert!(!源碼場地.join("README.md").exists());
        assert!(!源碼場地.join("LICENSE.txt").exists());
        assert!(!源碼場地.join("zh-hant-t-essay-bgw.gram").exists());

        // 驗證 STAGING_DIR (stage/)
        assert!(整備場地.join("zh-hant-t-essay-bgw.gram").exists());
        assert!(整備場地.join("rime.lua").exists());
        assert!(整備場地.join("opencc/t2s.json").exists());
        assert!(整備場地.join("lua/filter.lua").exists());
        assert!(!整備場地.join("luna_pinyin.schema.yaml").exists());
    }

    #[test]
    fn 測試安裝配方_多分支切換() {
        let 臨時場地 = tempfile::tempdir().unwrap();
        let 下載場地 = 臨時場地.path().join("pkg");
        let 源碼場地 = 臨時場地.path().join("src");
        let 整備場地 = 臨時場地.path().join("stage");

        let 倉庫目錄 = 下載場地.join("lotem").join("rime-octagram-data");
        fs::create_dir_all(&倉庫目錄).unwrap();
        let repo = git2::Repository::init(&倉庫目錄).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        // 在 master 分支提交 grammar.yaml
        fs::write(
            倉庫目錄.join("grammar.yaml"),
            "grammar:\n  language: zh-hant-t-essay-bgw\n",
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("grammar.yaml")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let master_commit_id = repo
            .commit(
                Some("refs/heads/master"),
                &sig,
                &sig,
                "init master",
                &tree,
                &[],
            )
            .unwrap();
        let master_commit = repo.find_commit(master_commit_id).unwrap();

        // 在 hant 分支提交 zh-hant-t-essay-bgw.gram
        repo.branch("hant", &master_commit, false).unwrap();
        repo.set_head("refs/heads/hant").unwrap();
        fs::remove_file(倉庫目錄.join("grammar.yaml")).unwrap();
        fs::write(倉庫目錄.join("zh-hant-t-essay-bgw.gram"), b"GRAM_DATA").unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("grammar.yaml")).unwrap();
        index
            .add_path(Path::new("zh-hant-t-essay-bgw.gram"))
            .unwrap();
        index.write().unwrap();
        let hant_tree_id = index.write_tree().unwrap();
        let hant_tree = repo.find_tree(hant_tree_id).unwrap();
        repo.commit(
            Some("refs/heads/hant"),
            &sig,
            &sig,
            "add gram on hant",
            &hant_tree,
            &[&master_commit],
        )
        .unwrap();

        // 1. 安裝默認分支 (master) 配置配方
        let 配置配方 = 配方名片::from("lotem/rime-octagram-data");
        assert_ok!(安裝配方(&配置配方, &下載場地, &源碼場地, &整備場地));
        assert!(源碼場地.join("grammar.yaml").exists());

        // 2. 安裝 @hant 分支模型數據配方
        let 模型配方 = 配方名片::from("lotem/rime-octagram-data@hant");
        assert_ok!(安裝配方(&模型配方, &下載場地, &源碼場地, &整備場地));
        assert!(整備場地.join("zh-hant-t-essay-bgw.gram").exists());
    }
}

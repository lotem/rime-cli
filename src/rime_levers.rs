use anyhow::{anyhow, bail};
use rime::{
    rime_api_call, rime_module_call, rime_struct_new, RimeConfig, RimeLeversApi, RimeTraits,
};
use std::ffi::{CStr, CString};
use std::path::PathBuf;

pub fn 設置引擎啓動參數(工作場地: &PathBuf) -> anyhow::Result<()> {
    log::debug!("設置引擎啓動參數. 工作場地: {}", 工作場地.display());
    std::fs::create_dir_all(工作場地)?;
    let 場地〇 = CString::new(工作場地.to_str().ok_or(anyhow!("路徑編碼轉換錯誤"))?)?;
    let 品名〇 = CString::new(env!("CARGO_PKG_NAME"))?;
    let 版本〇 = CString::new(env!("CARGO_PKG_VERSION"))?;
    let mut 啓動參數: RimeTraits = rime_struct_new!();
    啓動參數.data_size = std::mem::size_of::<RimeTraits>() as std::ffi::c_int;
    啓動參數.shared_data_dir = 場地〇.as_ptr();
    啓動參數.user_data_dir = 場地〇.as_ptr();
    啓動參數.distribution_name = 品名〇.as_ptr();
    啓動參數.distribution_code_name = 品名〇.as_ptr();
    啓動參數.distribution_version = 版本〇.as_ptr();
    rime_api_call!(setup, &mut 啓動參數);
    Ok(())
}

pub fn 製備輸入法固件() -> anyhow::Result<()> {
    log::debug!("製備輸入法固件");
    rime_api_call!(deployer_initialize, std::ptr::null_mut());
    rime_api_call!(deploy);
    rime_api_call!(finalize);
    Ok(())
}

pub fn 配置補丁(目標配置: &str, 紐: &str, 值: &str) -> anyhow::Result<()> {
    log::debug!("配置補丁: {目標配置}:/{紐} = {值}");

    let 目標配置〇 = CString::new(目標配置)?;
    let 紐〇 = CString::new(紐)?;
    let 值〇 = CString::new(值)?;

    let mut 值解析爲節點樹: RimeConfig = rime_struct_new!();
    if rime_api_call!(config_load_string, &mut 值解析爲節點樹, 值〇.as_ptr()) == 0 {
        bail!("無效的 YAML 值: {}", 值);
    }

    let levers_模塊名〇 = CString::new("levers")?;
    let levers = rime_api_call!(find_module, levers_模塊名〇.as_ptr());
    if levers.is_null() {
        bail!("沒有 levers 模塊");
    }

    let 配置工具名稱〇 = CString::new("rime-cli")?;
    let 自定義配置 = rime_module_call!(
        levers => RimeLeversApi,
        custom_settings_init,
        目標配置〇.as_ptr(),
        配置工具名稱〇.as_ptr()
    );

    // 可能已有自定義配置, 先加載
    rime_module_call!(levers => RimeLeversApi, load_settings, 自定義配置);
    // 生成補丁
    if rime_module_call!(
        levers => RimeLeversApi,
        customize_item,
        自定義配置,
        紐〇.as_ptr(),
        &mut 值解析爲節點樹
    ) != 0
    {
        rime_module_call!(levers => RimeLeversApi, save_settings, 自定義配置);
        log::info!("補丁打好了. {目標配置}:/{紐}");
    }

    rime_module_call!(levers => RimeLeversApi, custom_settings_destroy, 自定義配置);
    rime_api_call!(config_close, &mut 值解析爲節點樹);

    Ok(())
}

pub fn 加入輸入方案列表(衆輸入方案: &[String]) -> anyhow::Result<()> {
    log::debug!("加入輸入方案列表: {:#?}", 衆輸入方案);
    rime_api_call!(deployer_initialize, std::ptr::null_mut());

    let mut 自定義配置: RimeConfig = rime_struct_new!();
    let 默認配置的自定義〇 = CString::new("default.custom")?;
    rime_api_call!(
        user_config_open,
        默認配置的自定義〇.as_ptr(),
        &mut 自定義配置
    );
    let mut 既有方案 = vec![];
    let 方案列表〇 = CString::new("patch/schema_list")?;
    let 既有方案數 = rime_api_call!(config_list_size, &mut 自定義配置, 方案列表〇.as_ptr()) as u64;
    for i in 0..既有方案數 {
        let 列表項〇 = CString::new(format!("patch/schema_list/@{}/schema", i))?;
        let 方案 = rime_api_call!(config_get_cstring, &mut 自定義配置, 列表項〇.as_ptr());
        if !方案.is_null() {
            既有方案.push(unsafe { CStr::from_ptr(方案) }.to_str()?.to_owned());
        }
    }
    let 新增方案 = 衆輸入方案.iter().filter(|方案| !既有方案.contains(方案));
    let 新增列表項〇 = CString::new("patch/schema_list/@next/schema")?;
    for 方案 in 新增方案 {
        let 方案〇 = CString::new(方案.to_owned())?;
        rime_api_call!(
            config_set_string,
            &mut 自定義配置,
            新增列表項〇.as_ptr(),
            方案〇.as_ptr()
        );
    }
    rime_api_call!(config_close, &mut 自定義配置);

    rime_api_call!(finalize);
    Ok(())
}

pub fn 選擇輸入方案(方案: &str) -> anyhow::Result<()> {
    log::debug!("選擇輸入方案: {方案}");
    rime_api_call!(deployer_initialize, std::ptr::null_mut());

    let mut 用戶配置: RimeConfig = rime_struct_new!();
    let 用戶配置〇 = CString::new("user")?;
    rime_api_call!(user_config_open, 用戶配置〇.as_ptr(), &mut 用戶配置);
    let 用家之選〇 = CString::new("var/previously_selected_schema")?;
    let 方案〇 = CString::new(方案.to_owned())?;
    rime_api_call!(
        config_set_string,
        &mut 用戶配置,
        用家之選〇.as_ptr(),
        方案〇.as_ptr()
    );
    rime_api_call!(config_close, &mut 用戶配置);

    rime_api_call!(finalize);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use claims::assert_ok;
    use lazy_static::lazy_static;
    use std::fs::{read_to_string, write};
    use std::sync::{Once, RwLock};

    lazy_static! {
        static ref 公共測試場地: PathBuf = std::env::temp_dir().join("rime_levers_tests");
    }
    // 公共測試場地只需在各項測試開始之前清理一次.
    static 預備公共測試場地: Once = Once::new();
    // rime::Deployer 是個單例, 同一時刻只能服務一片場地.
    // 公共場地中的測試可以並發執行, 持讀鎖. 專用場地的測試持寫鎖.
    static 佔用引擎機位: RwLock<()> = RwLock::new(());

    fn 預備() {
        預備公共測試場地.call_once(|| {
            if 公共測試場地.exists() {
                assert_ok!(std::fs::remove_dir_all(&*公共測試場地));
            }
        });
        assert_ok!(設置引擎啓動參數(&公共測試場地));
    }

    #[test]
    fn 測試配置補丁_全局配置() {
        let _佔 = 佔用引擎機位.read().unwrap();
        預備();
        assert_ok!(配置補丁("default", "menu/page_size", "5"));

        let 結果文件 = 公共測試場地.join("default.custom.yaml");
        let 補丁文件內容 = assert_ok!(read_to_string(&結果文件));
        assert!(補丁文件內容.contains(
            r#"
patch:
  "menu/page_size": 5"#
        ));
    }

    #[test]
    fn 測試配置補丁_輸入方案() {
        let _佔 = 佔用引擎機位.read().unwrap();
        預備();
        assert_ok!(配置補丁("ohmyrime.schema", "menu/page_size", "9"));

        let 結果文件 = 公共測試場地.join("ohmyrime.custom.yaml");
        let 補丁文件內容 = assert_ok!(read_to_string(&結果文件));
        assert!(補丁文件內容.contains(
            r#"
patch:
  "menu/page_size": 9"#
        ));
    }

    #[test]
    fn 測試配置補丁_列表值() {
        let _佔 = 佔用引擎機位.read().unwrap();
        預備();
        assert_ok!(配置補丁(
            "patch_list",
            "starcraft/races",
            r#"[protoss, terran, zerg]"#
        ));

        let 結果文件 = 公共測試場地.join("patch_list.custom.yaml");
        let 補丁文件內容 = assert_ok!(read_to_string(&結果文件));
        println!("補丁文件內容: {}", 補丁文件內容);
        assert!(補丁文件內容.contains(
            r#"
patch:
  "starcraft/races":
    - protoss
    - terran
    - zerg"#
        ));
    }

    #[test]
    fn 測試配置補丁_字典值() {
        let _佔 = 佔用引擎機位.read().unwrap();
        預備();
        assert_ok!(配置補丁(
            "patch_map",
            "starcraft/workers",
            r#"{protoss: probe, terran: scv, zerg: drone}"#
        ));

        let 結果文件 = 公共測試場地.join("patch_map.custom.yaml");
        let 補丁文件內容 = assert_ok!(read_to_string(&結果文件));
        assert!(補丁文件內容.contains(
            r#"
patch:
  "starcraft/workers":
    protoss: probe
    terran: scv
    zerg: drone"#
        ));
    }

    #[test]
    fn 測試製備輸入法固件() {
        let _佔 = 佔用引擎機位.write().unwrap();
        let 專用測試場地 = std::env::temp_dir().join("rime_levers_tests_build");
        if 專用測試場地.exists() {
            assert_ok!(std::fs::remove_dir_all(&專用測試場地));
        }
        assert_ok!(設置引擎啓動參數(&專用測試場地));
        assert_ok!(write(
            專用測試場地.join("default.yaml"),
            r#"
schema_list:
  - schema: ohmyrime
"#,
        ));
        assert_ok!(write(
            專用測試場地.join("ohmyrime.schema.yaml"),
            r#"
schema:
  schema_id: ohmyrime
"#,
        ));

        assert_ok!(製備輸入法固件());

        assert!(專用測試場地.join("installation.yaml").exists());
        assert!(專用測試場地.join("user.yaml").exists());
        let 整備區 = 專用測試場地.join("build");
        let 默認配置文件 = 整備區.join("default.yaml");
        let 默認配置內容 = assert_ok!(read_to_string(&默認配置文件));
        assert!(默認配置內容.contains(
            r#"
schema_list:
  - schema: ohmyrime"#
        ));
        let 輸入方案文件 = 整備區.join("ohmyrime.schema.yaml");
        let 輸入方案內容 = assert_ok!(read_to_string(&輸入方案文件));
        assert!(輸入方案內容.contains(
            r#"
schema:
  schema_id: ohmyrime"#
        ));
    }

    #[test]
    fn 測試加入輸入方案列表() {
        let _佔 = 佔用引擎機位.write().unwrap();
        let 專用測試場地 = std::env::temp_dir().join("rime_levers_tests_add");
        if 專用測試場地.exists() {
            assert_ok!(std::fs::remove_dir_all(&專用測試場地));
        }
        assert_ok!(設置引擎啓動參數(&專用測試場地));

        let 新增輸入方案 = vec!["protoss".to_owned(), "terran".to_owned()];
        assert_ok!(加入輸入方案列表(&新增輸入方案));

        let 自定義配置 = 專用測試場地.join("default.custom.yaml");
        assert!(自定義配置.exists());
        let 自定義配置內容 = assert_ok!(read_to_string(&自定義配置));
        assert!(自定義配置內容.contains(
            r#"patch:
  schema_list:
    - {schema: protoss}
    - {schema: terran}"#
        ));

        let 新增輸入方案 = vec!["terran".to_owned(), "zerg".to_owned()];
        assert_ok!(加入輸入方案列表(&新增輸入方案));
        let 自定義配置內容 = assert_ok!(read_to_string(&自定義配置));
        assert!(自定義配置內容.contains(
            r#"patch:
  schema_list:
    - {schema: protoss}
    - {schema: terran}
    - {schema: zerg}"#
        ));
    }

    #[test]
    fn 測試選擇輸入方案() {
        let _佔 = 佔用引擎機位.write().unwrap();
        let 專用測試場地 = std::env::temp_dir().join("rime_levers_tests_select");
        if 專用測試場地.exists() {
            assert_ok!(std::fs::remove_dir_all(&專用測試場地));
        }
        assert_ok!(設置引擎啓動參數(&專用測試場地));

        let grrrr_之選 = "protoss";
        assert_ok!(選擇輸入方案(grrrr_之選));

        let 用戶配置 = 專用測試場地.join("user.yaml");
        assert!(用戶配置.exists());
        let 用戶配置內容 = assert_ok!(read_to_string(&用戶配置));
        assert!(用戶配置內容.contains(
            r#"var:
  previously_selected_schema: protoss"#
        ));

        let boxer_之選 = "terran";
        assert_ok!(選擇輸入方案(boxer_之選));

        let 用戶配置內容 = assert_ok!(read_to_string(&用戶配置));
        assert!(用戶配置內容.contains(
            r#"var:
  previously_selected_schema: terran"#
        ));
    }
}

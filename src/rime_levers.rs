use anyhow::bail;
use rime::{rime_api_call, rime_module_call, rime_struct_new, RimeConfig, RimeLeversApi};
use std::ffi::CString;

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

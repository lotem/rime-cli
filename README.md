# Rime 命令行工具

集成一部分 librime 的提供的工具，如數據部署工具、生成配置補丁等。
遠景是與 rime-make 配合，實現全功能的配方管理器，
幫助輸入方案設計者及用家以配方爲單位創作、分發、使用 Rime 輸入法的數據。
可用於命令行、持續集成環境中 Rime 方案及配置的編譯、離線部署。
也可當作以配方爲核心的圖形配置程序的後端。

## 構建腳本

``` shell
cargo make
```

以下是我在 Mac 上開發測試用的 `Makefile.toml` 配置文件。

構建 librime-sys 以及跑測試需要將以下路徑變量設爲 librime 成品目錄。
我本地的 librime 代碼庫與 rime-cli 同級, 所以要回上級目錄去找。

``` toml
[tasks.build.env]
LIBRIME_INCLUDE_DIR = "${CARGO_MAKE_WORKING_DIRECTORY}/../librime/dist/include"
LIBRIME_LIB_DIR = "${CARGO_MAKE_WORKING_DIRECTORY}/../librime/dist/lib"

[tasks.test.env]
DYLD_LIBRARY_PATH = "${CARGO_MAKE_WORKING_DIRECTORY}/../librime/dist/lib"
```



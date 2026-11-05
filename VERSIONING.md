# 版本號規範

Sapling Launcher 使用 `主版本.次版本.修訂號` 格式，例如 `0.2.1`，規則參考 [Semantic Versioning](https://semver.org/lang/zh-TW/)。

## 什麼時候加哪一位

| 版本位 | 什麼時候加 | 例子 |
|---|---|---|
| 主版本 | 項目進入新階段，或舊資料不能直接沿用 | `1.0.0`、`2.0.0` |
| 次版本 | 新增用戶看得到的功能或明顯的介面改版 | `0.1.0` → `0.2.0` |
| 修訂號 | 只修問題或小調整，沒有新功能 | `0.2.0` → `0.2.1` |

加一位時，右邊的位數歸零。

## 0.x 階段

`0.x.y` 是早期版本，功能和資料格式都可能改變，改動會寫在更新日誌裡。

## 1.0.0 的條件

`1.0.0` 是 Sapling 第一個基礎版本，要全部達成才發佈：

- 支援多個實例，可以新增、複製和刪除
- 支援 Fabric，以及至少一種 Forge 或 NeoForge
- 支援 1.19 以前的正式版
- 可以在啟動器內下載模組
- 支援啟動器更新檢查
- 發佈檔有數位簽章
- 錯誤提示和主要流程經過多個版本的實際使用
- `instance.json` 和 `config.json` 的格式穩定，日後升級不需要用戶手動處理

## 預覽版

正式發佈前可以用 `-beta.N`，例如 `0.3.0-beta.1`。功能未定時用 `-alpha.N`。預覽版在 GitHub 標記為 Pre-release。

## 發佈步驟

1. 更新下列檔案的版本號，四處要一致：
   - `app/package.json`
   - `app/src-tauri/tauri.conf.json`
   - `app/src-tauri/Cargo.toml`
   - `crates/core/Cargo.toml`
2. 在 [CHANGELOG.md](CHANGELOG.md) 寫下這個版本的改動。
3. 確認 `cargo test --workspace` 和 CI 通過。
4. 建立 `vX.Y.Z` 標籤和 GitHub Release，附上免安裝版。檔名必須以 `_portable.exe` 結尾，自動更新靠它尋找新版本。

已發佈的版本不修改；要修問題就發佈新的版本號。

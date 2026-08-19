//! 应用配置：全局快捷键 + 动作配方列表。存于 app_config_dir/config.json。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::actions::{Step, StepKind};

/// 便捷构造一个步骤（默认配置用）。
fn step(kind: StepKind) -> Step {
    Step { confirm: false, kind }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub name: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    /// 配方级人工确认开关：启用后，勾选了 confirm 的步骤执行前会询问（Enter 执行 / Esc 取消）。
    #[serde(default)]
    pub confirm: bool,
    /// 旧版字段：配方固定的圆盘位置列表。已迁移到 MenuConfig.slots，仅为兼容保留。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<u16>,
}

/// 圆盘菜单单个扇区（菜单按钮）的配置：绑定配方 + 外观。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuSlot {
    /// 绑定的配方索引（0 起）。缺省表示未绑定。
    #[serde(default)]
    pub recipe: Option<u16>,
    /// 菜单显示名称，覆盖配方名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// 扇区自定义颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// 扇区图标（emoji 或短文本）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 显示名称字号（px）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_size: Option<u16>,
    /// 显示名称颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_color: Option<String>,
}

fn slots_is_empty(slots: &[Option<MenuSlot>]) -> bool {
    slots.iter().all(Option::is_none)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuConfig {
    /// 圆盘直径（px）。
    #[serde(default = "default_menu_size")]
    pub size: u16,
    /// 扇区数量。
    #[serde(default = "default_menu_sectors")]
    pub sectors: u16,
    /// 是否显示配方名称标签。
    #[serde(default = "default_show_labels")]
    pub show_labels: bool,
    /// 按扇区索引排布的绑定配置，不做自动填充。
    #[serde(default, skip_serializing_if = "slots_is_empty")]
    pub slots: Vec<Option<MenuSlot>>,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            size: default_menu_size(),
            sectors: default_menu_sectors(),
            show_labels: default_show_labels(),
            slots: vec![
                Some(MenuSlot {
                    recipe: Some(0),
                    label: None,
                    color: None,
                    icon: None,
                    label_size: None,
                    label_color: None,
                }),
                Some(MenuSlot {
                    recipe: Some(1),
                    label: None,
                    color: None,
                    icon: None,
                    label_size: None,
                    label_color: None,
                }),
            ],
        }
    }
}

fn default_menu_size() -> u16 {
    400
}

fn default_menu_sectors() -> u16 {
    8
}

fn default_show_labels() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// 唤起圆盘菜单的全局快捷键（tauri-plugin-global-shortcut 格式）。
    pub global_hotkey: String,
    /// 唤起完整配方列表的全局快捷键。
    #[serde(default = "default_list_hotkey")]
    pub list_hotkey: String,
    /// 动作配方列表。
    pub recipes: Vec<Recipe>,
    /// 快捷键唤起的圆盘菜单设置。
    #[serde(default)]
    pub menu: MenuConfig,
}

fn default_list_hotkey() -> String {
    "Ctrl+Alt+L".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global_hotkey: "Ctrl+Alt+R".to_string(),
            list_hotkey: default_list_hotkey(),
            menu: MenuConfig::default(),
            recipes: vec![
                Recipe {
                    name: "发送选中代码到 Claude".to_string(),
                    slots: Vec::new(),
                    confirm: false,
                    steps: vec![
                        step(StepKind::Wait { ms: 50 }),
                        step(StepKind::Hotkey { keys: "Ctrl+C".to_string() }),
                        step(StepKind::Wait { ms: 150 }),
                        step(StepKind::ActivateApp {
                            title: "Claude".to_string(),
                            exe: None,
                        }),
                        step(StepKind::Wait { ms: 600 }),
                        step(StepKind::Hotkey { keys: "Ctrl+V".to_string() }),
                        step(StepKind::Wait { ms: 400 }),
                        step(StepKind::Hotkey { keys: "Enter".to_string() }),
                    ],
                },
                Recipe {
                    name: "发送文件路径到 Claude".to_string(),
                    slots: Vec::new(),
                    confirm: false,
                    steps: vec![
                        step(StepKind::Wait { ms: 50 }),
                        step(StepKind::Hotkey { keys: "Ctrl+Shift+C".to_string() }),
                        step(StepKind::Wait { ms: 150 }),
                        step(StepKind::ActivateApp {
                            title: "Claude".to_string(),
                            exe: None,
                        }),
                        step(StepKind::Wait { ms: 600 }),
                        step(StepKind::Hotkey { keys: "Ctrl+V".to_string() }),
                        step(StepKind::Wait { ms: 400 }),
                        step(StepKind::Hotkey { keys: "Enter".to_string() }),
                    ],
                },
            ],
        }
    }
}

pub fn config_path(app_config_dir: PathBuf) -> PathBuf {
    app_config_dir.join("config.json")
}

/// 把旧格式（Recipe.slots）迁移到 MenuConfig.slots，随后清空旧字段。
fn migrate(cfg: &mut Config) {
    let has_recipe_slots = cfg.recipes.iter().any(|r| !r.slots.is_empty());
    let has_menu_slots = cfg.menu.slots.iter().any(|s| s.is_some());
    if has_recipe_slots && !has_menu_slots {
        let sectors = cfg.menu.sectors.max(1) as usize;
        let mut slots: Vec<Option<MenuSlot>> = vec![None; sectors];
        for (ri, r) in cfg.recipes.iter().enumerate() {
            for &s in &r.slots {
                let s = s as usize;
                if s < sectors && slots[s].is_none() {
                    slots[s] = Some(MenuSlot {
                        recipe: Some(ri as u16),
                        label: None,
                        color: None,
                        icon: None,
                        label_size: None,
                        label_color: None,
                    });
                }
            }
        }
        cfg.menu.slots = slots;
    }
    for r in &mut cfg.recipes {
        r.slots.clear();
    }
}

pub fn load(app_config_dir: PathBuf) -> Config {
    let path = config_path(app_config_dir.clone());
    match fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str::<Config>(&s) {
            Ok(mut cfg) => {
                migrate(&mut cfg);
                let _ = save(&path, &cfg);
                cfg
            }
            Err(e) => {
                // 解析失败：先备份原文件（不覆盖用户数据），再从备份恢复，避免“配置全丢”。
                eprintln!("[config] PARSE FAILED: {e}");
                let _ = backup_corrupt(&path);
                recover_or_default(&path)
            }
        },
        Err(e) => {
            eprintln!("[config] READ FAILED: {e}");
            recover_or_default(&path)
        }
    }
}

/// 把疑似损坏/不可解析的配置文件改名为 .corrupt-<时间戳> 备份，防止被默认配置覆盖丢失。
fn backup_corrupt(path: &PathBuf) -> std::io::Result<()> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bak = path.with_extension(format!("json.corrupt-{ts}"));
    match fs::rename(path, &bak) {
        Ok(_) => Ok(()),
        Err(_) => fs::copy(path, &bak).map(|_| ()),
    }
}

/// 在当前目录里找最近一次的可恢复备份（backup_corrupt 生成的 config.json.corrupt-<ts>，
/// 兼容手工的 config.json.backup），按修改时间取最新。
fn find_newest_backup(path: &PathBuf) -> Option<PathBuf> {
    let dir = path.parent()?.to_path_buf();
    let stem = path.file_name()?.to_string_lossy().into_owned();
    let prefix = format!("{stem}.corrupt-");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.starts_with(&prefix) || name == format!("{stem}.backup") {
                cands.push(p);
            }
        }
    }
    cands.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    });
    cands.into_iter().rev().next()
}

/// 解析失败或文件缺失时的兜底：优先从最近的备份恢复配置；
/// 无可用备份才回退默认配置，并把结果写回主路径，确保 config.json 始终存在——
/// 避免“启动后像新软件、之前配置全没了”的观感。
fn recover_or_default(path: &PathBuf) -> Config {
    if let Some(bak) = find_newest_backup(path) {
        eprintln!("[config] trying backup: {}", bak.display());
        if let Ok(s) = fs::read_to_string(&bak) {
            if let Ok(mut cfg) = serde_json::from_str::<Config>(&s) {
                eprintln!(
                    "[config] recovered recipes = {}",
                    cfg.recipes.iter().map(|r| r.name.clone()).collect::<Vec<_>>().join(", ")
                );
                migrate(&mut cfg);
                let _ = save(path, &cfg);
                return cfg;
            }
        }
    }
    eprintln!("[config] no usable backup; using default config");
    let cfg = Config::default();
    let _ = save(path, &cfg);
    cfg
}

pub fn save(path: &PathBuf, cfg: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // 原子写入：先写临时文件再重命名，避免进程被中途终止时留下截断/损坏的 config.json，
    // 从源头杜绝“下次启动解析失败 → 配置看起来全没了”的问题。
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_moves_recipe_slots_to_menu() {
        let mut cfg = Config::default();
        // 旧格式：配方自带 slots，菜单无绑定 → 触发迁移。
        cfg.menu.slots = vec![];
        cfg.recipes[0].slots = vec![1, 3];
        cfg.recipes[1].slots = vec![3]; // 与配方 0 冲突，先到先得
        migrate(&mut cfg);
        assert_eq!(cfg.menu.slots.len(), cfg.menu.sectors as usize);
        assert_eq!(cfg.menu.slots[1].as_ref().unwrap().recipe, Some(0));
        assert_eq!(cfg.menu.slots[3].as_ref().unwrap().recipe, Some(0));
        assert!(cfg.menu.slots[0].is_none());
        // 旧字段已清空。
        assert!(cfg.recipes.iter().all(|r| r.slots.is_empty()));
    }

    #[test]
    fn migrate_skipped_when_menu_already_bound() {
        let mut cfg = Config::default();
        // 菜单已有绑定（默认配置即如此）→ 不迁移，只清旧字段。
        let before = cfg.menu.slots.clone();
        cfg.recipes[0].slots = vec![5];
        migrate(&mut cfg);
        assert_eq!(
            before.iter().map(|s| s.as_ref().and_then(|m| m.recipe)).collect::<Vec<_>>(),
            cfg.menu.slots.iter().map(|s| s.as_ref().and_then(|m| m.recipe)).collect::<Vec<_>>()
        );
        assert!(cfg.recipes.iter().all(|r| r.slots.is_empty()));
    }

    /// JSON 形态契约：前端 src/types/index.ts 依据本结构手写类型，
    /// 此处锁定序列化键名与步骤形态；改动本测试意味着需要同步前端类型。
    #[test]
    fn config_json_contract() {
        let v = serde_json::to_value(Config::default()).unwrap();
        assert!(v.get("globalHotkey").is_some());
        assert!(v.get("listHotkey").is_some());
        assert!(v.get("recipes").is_some());

        let menu = &v["menu"];
        assert!(menu.get("size").is_some());
        assert!(menu.get("sectors").is_some());
        assert!(menu.get("showLabels").is_some());
        assert!(menu.get("slots").is_some());

        let menu_slot = menu["slots"].as_array().unwrap().iter().find_map(|s| {
            let s = s.as_object()?;
            s.get("recipe").map(|_| s)
        });
        assert!(menu_slot.is_some(), "默认菜单应至少有一个绑定扇区");

        let recipe = &v["recipes"][0];
        assert!(recipe.get("name").is_some());
        assert!(recipe.get("steps").is_some());
        // 旧字段 slots 为空时不序列化。
        assert!(recipe.get("slots").is_none());

        // 步骤形态：tag 在 "type"，confirm 平铺顶层。
        let hotkey = &recipe["steps"][1];
        assert_eq!(hotkey["type"], "hotkey");
        assert_eq!(hotkey["keys"], "Ctrl+C");
        assert_eq!(hotkey["confirm"], false);
    }

    /// 旧版配置（步骤变体自带 confirm、menu.slots 为空）可以无损读入并迁移绑定。
    #[test]
    fn legacy_config_deserializes() {
        let raw = r#"{
            "globalHotkey": "Ctrl+Alt+R",
            "menu": { "size": 400, "sectors": 8, "showLabels": true, "slots": [] },
            "recipes": [{
                "name": "旧配方",
                "slots": [2],
                "steps": [
                    { "type": "wait", "ms": 50, "confirm": false },
                    { "type": "pasteText", "text": "hi", "confirm": true },
                    { "type": "click", "title": "A", "corner": "topLeft", "rx": 0.1, "ry": 0.2 }
                ]
            }]
        }"#;
        let mut cfg: Config = serde_json::from_str(raw).unwrap();
        assert_eq!(cfg.list_hotkey, "Ctrl+Alt+L"); // 默认补全
        migrate(&mut cfg);
        assert_eq!(cfg.menu.slots[2].as_ref().unwrap().recipe, Some(0));
        assert!(cfg.recipes[0].steps[1].confirm);
    }
}

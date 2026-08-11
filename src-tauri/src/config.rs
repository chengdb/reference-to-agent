//! 应用配置：全局快捷键 + 动作配方列表。存于 app_config_dir/config.json。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::actions::Step;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub name: String,
    #[serde(default)]
    pub steps: Vec<Step>,
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
    /// 唤起菜单的全局快捷键（tauri-plugin-global-shortcut 格式）。
    pub global_hotkey: String,
    /// 动作配方列表。
    pub recipes: Vec<Recipe>,
    /// 快捷键唤起的圆盘菜单设置。
    #[serde(default)]
    pub menu: MenuConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global_hotkey: "Ctrl+Alt+R".to_string(),
            menu: MenuConfig::default(),
            recipes: vec![
                Recipe {
                    name: "发送选中代码到 Claude".to_string(),
                    slots: Vec::new(),
                    steps: vec![
                        Step::Wait { ms: 50 },
                        Step::Hotkey { keys: "Ctrl+C".to_string() },
                        Step::Wait { ms: 150 },
                        Step::ActivateApp {
                            title: "Claude".to_string(),
                            exe: None,
                        },
                        Step::Wait { ms: 600 },
                        Step::Hotkey { keys: "Ctrl+V".to_string() },
                        Step::Wait { ms: 400 },
                        Step::Hotkey { keys: "Enter".to_string() },
                    ],
                },
                Recipe {
                    name: "发送文件路径到 Claude".to_string(),
                    slots: Vec::new(),
                    steps: vec![
                        Step::Wait { ms: 50 },
                        Step::Hotkey { keys: "Ctrl+Shift+C".to_string() },
                        Step::Wait { ms: 150 },
                        Step::ActivateApp {
                            title: "Claude".to_string(),
                            exe: None,
                        },
                        Step::Wait { ms: 600 },
                        Step::Hotkey { keys: "Ctrl+V".to_string() },
                        Step::Wait { ms: 400 },
                        Step::Hotkey { keys: "Enter".to_string() },
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
        Ok(s) => {
            let mut cfg: Config = serde_json::from_str(&s).unwrap_or_default();
            migrate(&mut cfg);
            let _ = save(&path, &cfg);
            cfg
        }
        Err(_) => {
            let cfg = Config::default();
            let _ = save(&path, &cfg);
            cfg
        }
    }
}

pub fn save(path: &PathBuf, cfg: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

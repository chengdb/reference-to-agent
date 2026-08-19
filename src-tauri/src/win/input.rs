//! 键盘/鼠标注入、坐标换算与修饰键等待。

use std::cell::RefCell;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEINPUT, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::actions::{AxisPos, Base, Unit};
use super::window::{find_window_by_title, window_rect, WindowRect};

thread_local! {
    /// Enigo 实例复用：新建有平台初始化开销，在调用线程上懒初始化一次后复用。
    static ENIGO: RefCell<Option<Enigo>> = RefCell::new(None);
}

/// 在线程本地的共享 Enigo 上执行操作；首次使用时初始化。
fn with_enigo<R>(f: impl FnOnce(&mut Enigo) -> Result<R, String>) -> Result<R, String> {
    ENIGO.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(Enigo::new(&Settings::default()).map_err(|e| e.to_string())?);
        }
        f(slot.as_mut().unwrap())
    })
}

pub struct CursorPos {
    pub x: i32,
    pub y: i32,
}

pub fn cursor_pos() -> Option<CursorPos> {
    let mut pt = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut pt) };
    if ok == 0 {
        None
    } else {
        Some(CursorPos { x: pt.x, y: pt.y })
    }
}

/// 将鼠标指针移动到屏幕指定坐标（用于把光标置于圆盘中心）。
pub fn set_cursor_pos(x: i32, y: i32) -> Result<(), String> {
    let ok = unsafe { SetCursorPos(x, y) };
    if ok == 0 {
        Err("设置鼠标位置失败".into())
    } else {
        Ok(())
    }
}

/// 在屏幕坐标 (x, y) 处点击一次（优先保证点击准确）。
/// 流程：记录原光标位置 → 真实移动到目标点 → 左键按下→抬起 → 延迟 → 移回原位置。
/// 这样点击发生时 GetCursorPos 即为目标点，任何依赖「当前光标位置」判定点击的应用
/// 都能正确识别；用户仅看到光标极短暂地闪到目标再回来（点击准确优先）。
pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    // 记录点击前的光标位置，点击后恢复。
    let prev = cursor_pos().ok_or("无法获取当前鼠标位置")?;

    // 真实移动到目标点，保证点击时系统光标就在目标坐标。
    set_cursor_pos(x, y)?;
    thread::sleep(Duration::from_millis(5));

    // 用 SendInput 注入纯按下/抬起（不带 MOVE），点击发生在当前光标位置（即目标点）。
    let click = |flags: u32| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let down = click(MOUSEEVENTF_LEFTDOWN);
    let up = click(MOUSEEVENTF_LEFTUP);
    let mut inputs = [down, up];
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32) };

    // 点击事件注入后，稍等片刻让目标处理 down/up，再移回原位。
    thread::sleep(Duration::from_millis(10));

    // 无论成败都恢复光标原位。
    let _ = set_cursor_pos(prev.x, prev.y);

    if sent != inputs.len() as u32 {
        return Err(format!("发送鼠标点击失败（仅成功 {sent}/{}）", inputs.len()));
    }
    Ok(())
}

/// 按标题找到窗口后，按「x/y 轴各自独立定位」换算成屏幕坐标并点击一次。
/// 例如 x=Left+50%、y=Bottom+8% → 水平居中、垂直沉底偏上，适合点击聊天输入框。
pub fn click_in_window(title: &str, x: AxisPos, y: AxisPos) -> Result<(), String> {
    let hwnd = find_window_by_title(title)
        .ok_or_else(|| format!("未找到“{title}”对应的窗口，无法定位点击位置"))?;
    let rect = window_rect(hwnd).ok_or("无法获取目标窗口矩形")?;
    let (px, py) = axis_to_screen(rect, x, y)?;
    click_at(px, py)
}

/// 把「x/y 轴各自独立定位」换算为屏幕像素坐标。
fn axis_to_screen(rect: WindowRect, x: AxisPos, y: AxisPos) -> Result<(i32, i32), String> {
    let w = rect.width();
    let h = rect.height();

    // 偏移量：百分比转像素；像素取整并夹取到 [0, span]，避免点出窗口。
    let offset = |v: f64, unit: Unit, span: i32, axis: &str| -> Result<i32, String> {
        match unit {
            Unit::Percent => Ok((span as f64 * v.clamp(0.0, 1.0)).round() as i32),
            Unit::Px => {
                if v < 0.0 {
                    Err(format!("{axis} 轴像素偏移不能为负: {v}"))
                } else {
                    Ok((v.round() as i32).clamp(0, span))
                }
            }
        }
    };

    let sx = match x.base {
        Base::Left => rect.left + offset(x.value, x.unit, w, "x")?,
        Base::Right => rect.right - offset(x.value, x.unit, w, "x")?,
        _ => return Err("x 轴基准必须是 left 或 right".into()),
    };
    let sy = match y.base {
        Base::Top => rect.top + offset(y.value, y.unit, h, "y")?,
        Base::Bottom => rect.bottom - offset(y.value, y.unit, h, "y")?,
        _ => return Err("y 轴基准必须是 top 或 bottom".into()),
    };
    Ok((sx, sy))
}

/// 返回光标位置相对目标窗口的 x/y 轴定位：各轴自动选择「距离较近的基准边」。
/// x_unit/y_unit 指定偏移量单位（percent/px），拾取结果按该单位返回，
/// 使拾取尊重用户在配置里已选择的单位。
pub fn cursor_ratio_in_window(
    title: &str,
    x_unit: Unit,
    y_unit: Unit,
) -> Result<(AxisPos, AxisPos), String> {
    let hwnd = find_window_by_title(title)
        .ok_or_else(|| format!("未找到“{title}”对应的窗口"))?;
    let rect = window_rect(hwnd).ok_or("无法获取目标窗口矩形")?;
    let cur = cursor_pos().ok_or("无法获取鼠标位置")?;
    let w = rect.width();
    let h = rect.height();
    if w <= 0 || h <= 0 {
        return Err("目标窗口尺寸异常".into());
    }
    let fx = (cur.x - rect.left) as f64 / w as f64; // 0=左 1=右
    let fy = (cur.y - rect.top) as f64 / h as f64; // 0=上 1=下

    // 基准右/下时用 (1 - 比例) 作为偏移；基准左/上直接用比例。
    // 像素单位则把比例乘上对应跨度。
    let to_val = |ratio: f64, span: i32, unit: Unit| -> f64 {
        match unit {
            Unit::Percent => ratio,
            Unit::Px => ratio * span as f64,
        }
    };

    let x = if fx >= 0.5 {
        AxisPos { base: Base::Right, value: to_val(1.0 - fx, w, x_unit), unit: x_unit }
    } else {
        AxisPos { base: Base::Left, value: to_val(fx, w, x_unit), unit: x_unit }
    };
    let y = if fy >= 0.5 {
        AxisPos { base: Base::Bottom, value: to_val(1.0 - fy, h, y_unit), unit: y_unit }
    } else {
        AxisPos { base: Base::Top, value: to_val(fy, h, y_unit), unit: y_unit }
    };
    Ok((x, y))
}

const ALPHA_KEYS: [Key; 26] = [
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K,
    Key::L, Key::M, Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V,
    Key::W, Key::X, Key::Y, Key::Z,
];

const DIGIT_KEYS: [Key; 10] = [
    Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7,
    Key::Num8, Key::Num9,
];

fn key_from_str(s: &str) -> Result<Key, String> {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "enter" | "return" => Ok(Key::Return),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "escape" | "esc" => Ok(Key::Escape),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "insert" => Ok(Key::Insert),
        "up" | "uparrow" => Ok(Key::UpArrow),
        "down" | "downarrow" => Ok(Key::DownArrow),
        "left" | "leftarrow" => Ok(Key::LeftArrow),
        "right" | "rightarrow" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        other => {
            let chars: Vec<char> = other.chars().collect();
            if chars.len() == 1 {
                let c = chars[0];
                if c.is_ascii_alphabetic() {
                    return Ok(ALPHA_KEYS[(c.to_ascii_uppercase() as u8 - b'A') as usize]);
                }
                if c.is_ascii_digit() {
                    return Ok(DIGIT_KEYS[(c as u8 - b'0') as usize]);
                }
                return Ok(Key::Unicode(c));
            }
            Err(format!("不支持的按键: {other}"))
        }
    }
}

/// 解析形如 "Ctrl+Shift+C" 的组合键并注入。
pub fn press_hotkey(combo: &str) -> Result<(), String> {
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key: Option<Key> = None;

    for part in combo.split('+') {
        let p = part.trim();
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "win" | "meta" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            _ => key = Some(key_from_str(p)?),
        }
    }
    let key = key.ok_or_else(|| format!("组合键缺少主键: {combo}"))?;

    with_enigo(|enigo| {
        // 依次按下，最后无论成败都释放修饰键，避免键位卡住。
        let r1 = if ctrl {
            enigo.key(Key::Control, Direction::Press)
        } else {
            Ok(())
        };
        let r2 = if alt {
            enigo.key(Key::Alt, Direction::Press)
        } else {
            Ok(())
        };
        let r3 = if shift {
            enigo.key(Key::Shift, Direction::Press)
        } else {
            Ok(())
        };
        let r4 = enigo.key(key, Direction::Press);
        let r5 = enigo.key(key, Direction::Release);
        let _ = enigo.key(Key::Shift, Direction::Release);
        let _ = enigo.key(Key::Alt, Direction::Release);
        let _ = enigo.key(Key::Control, Direction::Release);

        r1.and(r2)
            .and(r3)
            .and(r4)
            .and(r5)
            .map_err(|e| format!("按键注入失败: {e}"))
    })
}

/// 直接注入一段文本（受当前输入法影响）。
pub fn type_text(text: &str) -> Result<(), String> {
    with_enigo(|enigo| enigo.text(text).map_err(|e| e.to_string()))
}

/// 若残留按下过 Shift（按下 Shift+Enter 确认全部时），等它松开后再放行下一步，避免
/// 残留修饰键污染下一步注入（Alt+L 复制会被当成 Alt+Shift+L、导致复制失败，表现为
/// “回滚剪切板先于粘贴、粘贴到的是旧剪贴板”）。
///
/// 只等真正会污染组合键的 Shift：
/// - 普通 Enter 确认时此刻 Shift 未按下，立即返回（零延迟）；
/// - Shift+Enter 确认全部时此刻 Shift 仍按下，等待其松开（正常 ~几十到一两百毫秒），
///   并带上限（最多约 400ms）保险，避免长时间阻塞造成可按觉的延迟。
/// 不等待 Enter 本身——Enter 不是修饰键，残留它不会把 Alt+L/Shift+Insert 错拼成别的键。
pub fn wait_for_shift_released() {
    // SAFETY: GetAsyncKeyState 是纯查询函数，无未定义行为要求。
    let shift_down = unsafe { GetAsyncKeyState(VK_SHIFT as i32) as i32 & 0x8000 != 0 };
    if !shift_down {
        return;
    }
    crate::debug_log!("wait_for_shift_released: waiting for Shift up");
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(50));
        // SAFETY: 同上。
        if unsafe { GetAsyncKeyState(VK_SHIFT as i32) as i32 & 0x8000 == 0 } {
            crate::debug_log!("wait_for_shift_released: released");
            return;
        }
    }
    crate::debug_log!("wait_for_shift_released: gave up on timeout");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> WindowRect {
        // 1000x500 的窗口，左上角 (100, 200)。
        WindowRect { left: 100, top: 200, right: 1100, bottom: 700 }
    }

    #[test]
    fn axis_to_screen_percent() {
        let (x, y) = axis_to_screen(
            rect(),
            AxisPos::percent(Base::Left, 0.5),
            AxisPos::percent(Base::Bottom, 0.1),
        )
        .unwrap();
        assert_eq!(x, 600); // 100 + 1000*0.5
        assert_eq!(y, 650); // 700 - 500*0.1
    }

    #[test]
    fn axis_to_screen_right_top() {
        let (x, y) = axis_to_screen(
            rect(),
            AxisPos::percent(Base::Right, 0.25),
            AxisPos::percent(Base::Top, 0.0),
        )
        .unwrap();
        assert_eq!(x, 850); // 1100 - 1000*0.25
        assert_eq!(y, 200);
    }

    #[test]
    fn axis_to_screen_px_clamped_into_window() {
        let pos = |base: Base, value: f64, unit: Unit| AxisPos { base, value, unit };
        // 超出跨度的像素偏移被夹取到窗口边缘。
        let (x, y) = axis_to_screen(
            rect(),
            pos(Base::Left, 5000.0, Unit::Px),
            pos(Base::Bottom, 9999.0, Unit::Px),
        )
        .unwrap();
        assert_eq!(x, 1100);
        assert_eq!(y, 200);
    }

    #[test]
    fn axis_to_screen_negative_px_rejected() {
        let pos = |base: Base, value: f64, unit: Unit| AxisPos { base, value, unit };
        let err = axis_to_screen(rect(), pos(Base::Left, -1.0, Unit::Px), pos(Base::Top, 0.0, Unit::Px))
            .unwrap_err();
        assert!(err.contains("不能为负"));
    }

    #[test]
    fn axis_to_screen_wrong_base_rejected() {
        let pos = |base: Base, value: f64, unit: Unit| AxisPos { base, value, unit };
        assert!(axis_to_screen(rect(), pos(Base::Top, 0.0, Unit::Percent), pos(Base::Top, 0.0, Unit::Percent)).is_err());
        assert!(axis_to_screen(rect(), pos(Base::Left, 0.0, Unit::Percent), pos(Base::Left, 0.0, Unit::Percent)).is_err());
    }
}

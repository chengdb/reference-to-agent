//! 剪贴板读写。

use std::thread;
use std::time::Duration;

/// 写入剪贴板文本。
pub fn write_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .map_err(|e| e.to_string())
        .and_then(|mut c| c.set_text(text.to_string()).map_err(|e| e.to_string()))
}

/// 读取剪贴板文本。
/// - `Ok(Some(text))`：当前是文本内容（含空字符串文本）。
/// - `Ok(None)`：剪贴板为空或非文本（图片/文件等），无法还原为文本。
/// - `Err(e)`：剪贴板被占用等，短暂重试仍失败。
pub fn read_clipboard() -> Result<Option<String>, String> {
    let mut clip = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    // 剪贴板可能被其他进程短暂占用，重试几次缓解。
    for _ in 0..3 {
        match clip.get_text() {
            Ok(text) => return Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => return Ok(None),
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
    // 若最终失败，保守返回 None（视为不可还原），回滚成为 no-op，
    // 避免误写空字符串覆盖用户的非文本剪贴板。
    Ok(None)
}

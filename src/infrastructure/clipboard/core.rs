use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static WRITING: AtomicBool = AtomicBool::new(false);

struct WriteGuard;
impl Drop for WriteGuard {
    fn drop(&mut self) {
        WRITING.store(false, Ordering::Release);
    }
}

pub(super) fn write_text(text: String) -> Result<(), String> {
    if WRITING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("上一次剪贴板写入尚未完成".to_owned());
    }
    let _writing = WriteGuard;
    let started = Instant::now();
    log::info!(
        "clipboard_write_started bytes={} thread=blocking backend=arboard",
        text.len()
    );
    let result = write_native(&text);
    log::info!(
        "clipboard_write_finished success={} elapsed_ms={}",
        result.is_ok(),
        started.elapsed().as_millis()
    );
    result.map_err(|error| format!("写入系统剪贴板失败: {error}"))
}

fn write_native(text: &str) -> Result<(), arboard::Error> {
    // Construct, use and drop on this worker; never borrow the GUI platform.
    let mut clipboard = arboard::Clipboard::new()?;
    let mut retries = 0;
    loop {
        match clipboard.set_text(text) {
            Ok(()) => return Ok(()),
            Err(arboard::Error::ClipboardOccupied) if retries < 10 => {
                retries += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error),
        }
    }
}

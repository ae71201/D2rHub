use image::RgbImage;
#[cfg(windows)]
pub(super) fn capture(pid: u32) -> Result<RgbImage, String> {
    use std::sync::mpsc;
    use windows_capture::{
        capture::{Context, GraphicsCaptureApiHandler},
        frame::Frame,
        graphics_capture_api::InternalCaptureControl,
        settings::*,
        window::Window,
    };
    type Error = Box<dyn std::error::Error + Send + Sync>;
    struct Shot {
        sender: mpsc::SyncSender<RgbImage>,
    }
    impl GraphicsCaptureApiHandler for Shot {
        type Flags = mpsc::SyncSender<RgbImage>;
        type Error = Error;
        fn new(ctx: Context<Self::Flags>) -> Result<Self, Error> {
            Ok(Self { sender: ctx.flags })
        }
        fn on_frame_arrived(
            &mut self,
            frame: &mut Frame,
            control: InternalCaptureControl,
        ) -> Result<(), Error> {
            let buffer = frame.buffer_without_title_bar()?;
            let (w, h) = (buffer.width(), buffer.height());
            if w < 100 || h < 100 || u64::from(w) * u64::from(h) > 40_000_000 {
                control.stop();
                return Ok(());
            }
            let mut packed = Vec::new();
            let rgba = buffer.as_nopadding_buffer(&mut packed);
            let rgb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect();
            if let Some(image) = RgbImage::from_raw(w, h, rgb) {
                let _ = self.sender.try_send(image);
            }
            control.stop();
            Ok(())
        }
    }
    let hwnd = crate::infrastructure::system::find_game_hwnd(pid).ok_or("目标游戏窗口不存在")?;
    unsafe {
        if windows::Win32::UI::WindowsAndMessaging::IsIconic(windows::Win32::Foundation::HWND(
            hwnd as *mut _,
        ))
        .as_bool()
        {
            return Err("目标游戏已最小化，无法核验".into());
        }
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let settings = Settings::new(
        Window::from_raw_hwnd(hwnd as *mut _),
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        tx,
    );
    let control = Shot::start_free_threaded(settings).map_err(|e| e.to_string())?;
    let result = rx
        .recv_timeout(std::time::Duration::from_millis(1200))
        .map_err(|_| "截图超时或窗口未刷新".to_string());
    control.stop().map_err(|e| e.to_string())?;
    let image = result?;
    if image
        .pixels()
        .step_by(97)
        .all(|p| p.0.iter().all(|c| *c < 5))
    {
        return Err("截图为空或黑帧，无法核验".into());
    }
    Ok(image)
}
#[cfg(not(windows))]
pub(super) fn capture(_pid: u32) -> Result<RgbImage, String> {
    Err("视觉识别仅支持 Windows".into())
}

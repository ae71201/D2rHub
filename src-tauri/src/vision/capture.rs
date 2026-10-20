//! Bounded, demand-driven WGC sessions. Pixels are read back only for a requested region.
use super::config::Roi;
use image::RgbImage;
use serde::Serialize;

#[cfg(all(test, windows))]
mod legacy;
#[cfg(windows)]
mod windows;

#[derive(Debug, Default, Serialize)]
pub struct CaptureMetrics {
    pub reused: bool,
    pub minimum_update_interval_ms: Option<u64>,
    pub queue_ms: f64,
    pub setup_ms: f64,
    pub wait_frame_ms: f64,
    pub readback_ms: f64,
    pub convert_ms: f64,
    pub total_ms: f64,
    pub frame_age_ms: f64,
    pub client_width: u32,
    pub client_height: u32,
    pub output_width: u32,
    pub output_height: u32,
}
pub struct Captured {
    pub image: RgbImage,
    pub metrics: CaptureMetrics,
}

pub fn capture(pid: u32, roi: Option<Roi>) -> Result<Captured, String> {
    if roi.is_some_and(|r| !r.valid()) {
        return Err("截图区域无效".into());
    }
    #[cfg(windows)]
    {
        windows::capture(pid, roi)
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, roi);
        Err("视觉识别仅支持 Windows".into())
    }
}

/// Flushes persistent sessions when the visual worker is stopped or reconfigured.
pub fn release_sessions() {
    #[cfg(windows)]
    windows::release_sessions();
}

fn region(width: u32, height: u32, roi: Option<Roi>) -> Result<(u32, u32, u32, u32), String> {
    if width < 100 || height < 100 || u64::from(width) * u64::from(height) > 40_000_000 {
        return Err("游戏窗口截图尺寸无效".into());
    }
    let r = roi.unwrap_or(Roi {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    if !r.valid() {
        return Err("截图区域无效".into());
    }
    let x = (r.x * width as f32) as u32;
    let y = (r.y * height as f32) as u32;
    if x >= width || y >= height {
        return Err("截图区域越界".into());
    }
    let w = ((r.width * width as f32) as u32).max(1).min(width - x);
    let h = ((r.height * height as f32) as u32).max(1).min(height - y);
    Ok((x, y, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roi_pixel_bounds_preserve_calibration_and_reject_invalid_frames() {
        assert_eq!(
            region(
                1920,
                1080,
                Some(Roi {
                    x: 0.0,
                    y: 0.4,
                    width: 0.31,
                    height: 0.4
                })
            )
            .unwrap(),
            (0, 432, 595, 432)
        );
        assert_eq!(
            region(
                1920,
                1080,
                Some(Roi {
                    x: 0.25,
                    y: 0.13,
                    width: 0.5,
                    height: 0.2
                })
            )
            .unwrap(),
            (480, 140, 960, 216)
        );
        assert!(region(0, 100, None).is_err());
        assert!(region(80000, 80000, None).is_err());
        assert!(region(
            1920,
            1080,
            Some(Roi {
                x: f32::NAN,
                y: 0.0,
                width: 0.5,
                height: 0.5
            })
        )
        .is_err());
    }
}

#[cfg(all(test, windows))]
mod benchmark {
    use super::*;
    #[test]
    #[ignore = "requires D2RHUB_CAPTURE_BENCH_PID for a running non-minimized D2R window; read-only"]
    fn compare_legacy_and_reused_gpu_roi_capture() {
        let pid: u32 = std::env::var("D2RHUB_CAPTURE_BENCH_PID")
            .unwrap()
            .parse()
            .unwrap();
        let roi = crate::vision::config::VisionConfig::default().chat_roi;
        let mut baseline = Vec::new();
        for _ in 0..4 {
            let start = std::time::Instant::now();
            let image = legacy::capture(pid).unwrap();
            let crop = crate::vision::processing::crop(&image, roi);
            assert!(crop.width() > 0);
            baseline.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let mut metrics = Vec::new();
        for _ in 0..4 {
            let captured = capture(pid, Some(roi)).unwrap();
            metrics.push(captured.metrics);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
        assert!(!metrics[0].reused);
        assert!(metrics[1..].iter().all(|m| m.reused));
        assert!(metrics
            .iter()
            .all(|m| m.output_width < m.client_width && m.output_height < m.client_height));
        windows::assert_idle_has_no_readbacks();
        release_sessions();
        println!(
            "CAPTURE_BENCH {}",
            serde_json::json!({"legacy_full_frame_then_crop_ms":baseline,"gpu_roi_capture":metrics})
        );
    }
}

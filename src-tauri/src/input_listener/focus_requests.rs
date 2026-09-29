//! One active shortcut operation and one replaceable pending request.
use super::*;

struct Pending<T> {
    running: bool,
    latest: Option<T>,
}

impl<T> Pending<T> {
    fn submit(&mut self, request: T) -> bool {
        self.latest = Some(request);
        let start = !self.running;
        self.running = true;
        start
    }

    fn next(&mut self) -> Option<T> {
        let request = self.latest.take();
        if request.is_none() {
            self.running = false;
        }
        request
    }
}

static REQUESTS: Mutex<Pending<(AppHandle, String)>> = Mutex::new(Pending {
    running: false,
    latest: None,
});

/// Called by the hotkey message pump, before spawning any operation threads,
/// so rapid presses retain their arrival order. No window or file I/O here.
pub(super) fn submit(app: AppHandle, shortcut: String) {
    let mut requests = REQUESTS.lock().unwrap_or_else(|error| error.into_inner());
    if !requests.submit((app, shortcut)) {
        return;
    }
    if let Err(error) = std::thread::Builder::new()
        .name("window-focus".into())
        .spawn(|| loop {
            let next = REQUESTS
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .next();
            let Some((app, shortcut)) = next else { break };
            // A failed operation must not leave the queue permanently marked busy.
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                dispatch_registered_shortcut(&app, &shortcut);
            }))
            .is_err()
            {
                crate::logger::log_msg("ERROR", "Shortcut", "窗口切换任务异常，继续处理后续请求");
            }
        })
    {
        requests.running = false;
        requests.latest = None;
        crate::logger::log_msg(
            "ERROR",
            "Shortcut",
            &format!("无法启动窗口切换任务：{error}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::Pending;

    #[test]
    fn focus_requests_keep_last_press_and_restart_after_idle() {
        let mut queue = Pending {
            running: false,
            latest: None,
        };
        assert!(queue.submit("A"));
        assert_eq!(queue.next(), Some("A"));
        assert!(!queue.submit("B"));
        assert!(!queue.submit("C"));
        assert_eq!(queue.next(), Some("C"));
        assert!(!queue.submit("D"));
        assert_eq!(queue.next(), Some("D"));
        assert_eq!(queue.next(), None);
        assert!(queue.submit("E"));
        assert_eq!(queue.next(), Some("E"));
    }
}

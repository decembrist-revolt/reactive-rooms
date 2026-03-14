use std::time::Duration;
use tokio::time::{Instant, Interval, interval};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct PingTracker {
    pub interval: Interval,
    deadline: Option<Instant>,
}

impl PingTracker {
    pub async fn new() -> Self {
        let mut iv = interval(PING_INTERVAL);
        iv.tick().await;
        Self {
            interval: iv,
            deadline: None,
        }
    }

    pub fn on_pong(&mut self) {
        self.deadline = None;
    }

    pub fn on_tick(&mut self) -> bool {
        if let Some(deadline) = self.deadline
            && Instant::now() > deadline
        {
            return false;
        }
        self.deadline = Some(Instant::now() + PONG_TIMEOUT);
        true
    }
}

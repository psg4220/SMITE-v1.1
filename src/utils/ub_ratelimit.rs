/// UnbelievaBoat API rate limiter - 20 requests per second globally
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::collections::VecDeque;

lazy_static! {
    static ref UB_RATE_LIMITER: Mutex<UbRateLimiter> = Mutex::new(UbRateLimiter::new());
}

pub struct UbRateLimiter {
    /// Queue of request timestamps (last 1 second)
    request_times: VecDeque<Instant>,
    /// Max requests per second
    max_requests: usize,
    /// Time window (1 second)
    window: Duration,
}

impl UbRateLimiter {
    fn new() -> Self {
        Self {
            request_times: VecDeque::new(),
            max_requests: 20,
            window: Duration::from_secs(1),
        }
    }

    fn check_and_record(&mut self) -> Duration {
        let now = Instant::now();
        
        // Remove old timestamps outside the 1-second window
        while let Some(&front) = self.request_times.front() {
            if now.duration_since(front) > self.window {
                self.request_times.pop_front();
            } else {
                break;
            }
        }

        // If we're at the limit, calculate how long to wait
        if self.request_times.len() >= self.max_requests {
            if let Some(&oldest) = self.request_times.front() {
                let elapsed = now.duration_since(oldest);
                if elapsed < self.window {
                    let wait_time = self.window - elapsed;
                    return wait_time;
                }
            }
        }

        // Record this request
        self.request_times.push_back(now);
        Duration::from_secs(0)
    }
}

/// Wait if necessary to respect the 20 requests/second rate limit for UnbelievaBoat API
pub async fn rate_limit_ub_api() {
    let wait_duration = {
        let mut limiter = UB_RATE_LIMITER.lock().unwrap();
        limiter.check_and_record()
    };

    if wait_duration.as_millis() > 0 {
        tracing::debug!("UB API rate limit: waiting {}ms", wait_duration.as_millis());
        tokio::time::sleep(wait_duration).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_requests_within_limit() {
        let mut limiter = UbRateLimiter::new();
        
        // First 20 requests should not require waiting
        for _ in 0..20 {
            let wait = limiter.check_and_record();
            assert_eq!(wait.as_millis(), 0);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut limiter = UbRateLimiter::new();
        
        // Fill up to 20 requests
        for _ in 0..20 {
            limiter.check_and_record();
        }
        
        // 21st request should require waiting
        let wait = limiter.check_and_record();
        assert!(wait.as_millis() > 0);
    }
}

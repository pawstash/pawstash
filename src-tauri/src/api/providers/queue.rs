use reqwest::{header::RETRY_AFTER, Response, StatusCode};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep, Instant};

#[derive(Debug, Clone)]
pub struct ProviderQueueConfig {
    pub max_concurrent: usize,
    pub min_interval: Duration,
    pub max_retries: usize,
    pub default_retry_after: Duration,
    pub max_cooldown: Duration,
}

impl Default for ProviderQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            min_interval: Duration::from_millis(100),
            max_retries: 3,
            default_retry_after: Duration::from_millis(1500),
            max_cooldown: Duration::from_secs(10),
        }
    }
}

impl ProviderQueueConfig {
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max.max(1);
        self
    }

    pub fn with_min_interval(mut self, interval: Duration) -> Self {
        self.min_interval = interval;
        self
    }

    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
}

#[derive(Debug)]
struct QueueGate {
    last_request_at: Option<Instant>,
    cooldown_until: Option<Instant>,
}

pub struct ProviderRequestQueue {
    provider_id: String,
    config: ProviderQueueConfig,
    semaphore: Arc<Semaphore>,
    gate: Arc<Mutex<QueueGate>>,
}

impl ProviderRequestQueue {
    pub fn new(provider_id: impl Into<String>, config: ProviderQueueConfig) -> Self {
        let max_concurrent = config.max_concurrent.max(1);
        Self {
            provider_id: provider_id.into(),
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            gate: Arc::new(Mutex::new(QueueGate {
                last_request_at: None,
                cooldown_until: None,
            })),
        }
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn config(&self) -> &ProviderQueueConfig {
        &self.config
    }

    pub async fn send_request<F>(&self, make_builder: F) -> Result<Response, String>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        self.send(move || {
            let req = make_builder();
            async move { req.send().await }
        })
        .await
    }

    pub async fn send<F, Fut>(&self, make_request: F) -> Result<Response, String>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<Response, reqwest::Error>>,
    {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| format!("Queue for '{}' closed: {e}", self.provider_id))?;

        let mut attempt = 0;
        loop {
            self.wait_for_slot().await;

            let resp = match make_request().await {
                Ok(r) => r,
                Err(err) => {
                    if (err.is_connect() || err.is_timeout()) && attempt < self.config.max_retries {
                        attempt += 1;
                        let delay = self
                            .config
                            .default_retry_after
                            .min(self.config.max_cooldown);
                        self.set_cooldown(delay).await;
                        tracing::warn!(
                            provider = %self.provider_id,
                            attempt,
                            error = %err,
                            "Transient network error; retrying after cooldown"
                        );
                        sleep(delay).await;
                        continue;
                    }
                    return Err(format!(
                        "Provider '{}' request error: {err}",
                        self.provider_id
                    ));
                }
            };

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < self.config.max_retries {
                attempt += 1;
                let delay = self.calculate_cooldown(&resp, attempt);
                self.set_cooldown(delay).await;
                tracing::warn!(
                    provider = %self.provider_id,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Provider received HTTP 429 Too Many Requests; applying queue cooldown"
                );
                sleep(delay).await;
                continue;
            }

            if (status == StatusCode::BAD_GATEWAY
                || status == StatusCode::SERVICE_UNAVAILABLE
                || status == StatusCode::GATEWAY_TIMEOUT)
                && attempt < self.config.max_retries
            {
                attempt += 1;
                let delay = self.calculate_cooldown(&resp, attempt);
                self.set_cooldown(delay).await;
                tracing::warn!(
                    provider = %self.provider_id,
                    status = %status,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Transient server error; applying queue cooldown and retrying"
                );
                sleep(delay).await;
                continue;
            }

            return Ok(resp);
        }
    }

    pub async fn wait_for_slot(&self) {
        loop {
            let sleep_duration = {
                let mut gate = self.gate.lock().await;
                let now = Instant::now();

                if let Some(cooldown) = gate.cooldown_until {
                    if cooldown > now {
                        Some(cooldown - now)
                    } else {
                        gate.cooldown_until = None;
                        if let Some(last) = gate.last_request_at {
                            if now < last + self.config.min_interval {
                                Some((last + self.config.min_interval) - now)
                            } else {
                                gate.last_request_at = Some(now);
                                None
                            }
                        } else {
                            gate.last_request_at = Some(now);
                            None
                        }
                    }
                } else if let Some(last) = gate.last_request_at {
                    if now < last + self.config.min_interval {
                        Some((last + self.config.min_interval) - now)
                    } else {
                        gate.last_request_at = Some(now);
                        None
                    }
                } else {
                    gate.last_request_at = Some(now);
                    None
                }
            };

            if let Some(dur) = sleep_duration {
                sleep(dur).await;
            } else {
                break;
            }
        }
    }

    fn calculate_cooldown(&self, resp: &Response, attempt: usize) -> Duration {
        if let Some(header) = resp.headers().get(RETRY_AFTER) {
            if let Ok(val) = header.to_str() {
                if let Ok(secs) = val.parse::<u64>() {
                    return Duration::from_secs(secs).min(self.config.max_cooldown);
                }
            }
        }
        let progressive = self.config.default_retry_after * attempt as u32;
        progressive.min(self.config.max_cooldown)
    }

    pub async fn set_cooldown(&self, delay: Duration) {
        let mut gate = self.gate.lock().await;
        let target = Instant::now() + delay;
        match gate.cooldown_until {
            Some(existing) if existing >= target => {}
            _ => gate.cooldown_until = Some(target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_provider_queue_concurrency_and_spacing() {
        let config = ProviderQueueConfig {
            max_concurrent: 2,
            min_interval: Duration::from_millis(50),
            max_retries: 2,
            default_retry_after: Duration::from_millis(100),
            max_cooldown: Duration::from_millis(500),
        };
        let queue = Arc::new(ProviderRequestQueue::new("test_prov", config));

        let active_concurrent = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for _ in 0..6 {
            let q = queue.clone();
            let active = active_concurrent.clone();
            let max_obs = max_observed.clone();
            let comp = completed.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = q.semaphore.acquire().await.unwrap();
                q.wait_for_slot().await;

                let cur = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_obs.fetch_max(cur, Ordering::SeqCst);
                sleep(Duration::from_millis(30)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                comp.fetch_add(1, Ordering::SeqCst);
            }));
        }

        for t in tasks {
            t.await.unwrap();
        }

        assert_eq!(completed.load(Ordering::SeqCst), 6);
        assert!(
            max_observed.load(Ordering::SeqCst) <= 2,
            "Concurrency exceeded max_concurrent limit: {}",
            max_observed.load(Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn test_provider_queues_are_completely_isolated() {
        let q_coomer = ProviderRequestQueue::new(
            "coomer",
            crate::api::providers::CoomerProvider::default_queue_config(),
        );
        let q_pawchive = ProviderRequestQueue::new(
            "pawchive",
            crate::api::providers::PawchiveProvider::default_queue_config(),
        );

        q_coomer.set_cooldown(Duration::from_secs(5)).await;

        let coomer_gate = q_coomer.gate.lock().await;
        assert!(coomer_gate.cooldown_until.is_some());
        drop(coomer_gate);

        let pawchive_gate = q_pawchive.gate.lock().await;
        assert!(pawchive_gate.cooldown_until.is_none());
        drop(pawchive_gate);

        let start = Instant::now();
        q_pawchive.wait_for_slot().await;
        assert!(start.elapsed() < Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_all_providers_have_isolated_queues_and_can_be_tweaked() {
        use crate::api::providers::coomer::CoomerProvider;
        use crate::api::providers::onlyhaven::OnlyHavenProvider;
        use crate::api::providers::pawchive::PawchiveProvider;
        use crate::api::providers::traits::ProviderConfig;

        let dummy_conf = |id: &str| ProviderConfig {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            priority: 1,
            api_url: "https://127.0.0.1:9".to_string(),
            fallback_urls: vec![],
            file_url: None,
            image_url: None,
            session_cookie: "".to_string(),
            username: "".to_string(),
            services: vec![],
            file_prefix: None,
            image_prefix: None,
            is_custom: false,
        };

        let coomer = CoomerProvider::new(dummy_conf("coomer")).unwrap();
        let onlyhaven = OnlyHavenProvider::new(dummy_conf("onlyhaven")).unwrap();
        let pawchive = PawchiveProvider::new(dummy_conf("pawchive")).unwrap();

        assert_eq!(coomer.queue().provider_id(), "coomer");
        assert_eq!(onlyhaven.queue().provider_id(), "onlyhaven");
        assert_eq!(pawchive.queue().provider_id(), "pawchive");

        assert_eq!(coomer.queue().config().max_concurrent, 2);
        assert_eq!(onlyhaven.queue().config().max_concurrent, 3);
        assert_eq!(pawchive.queue().config().max_concurrent, 4);

        let custom_cfg = ProviderQueueConfig {
            max_concurrent: 10,
            min_interval: Duration::from_millis(5),
            max_retries: 1,
            default_retry_after: Duration::from_millis(50),
            max_cooldown: Duration::from_millis(100),
        };
        let custom_coomer =
            CoomerProvider::with_queue_config(dummy_conf("coomer"), custom_cfg).unwrap();
        assert_eq!(custom_coomer.queue().config().max_concurrent, 10);
    }
}

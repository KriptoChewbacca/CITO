//! Rate limiting and circuit breaker implementations

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

// ============================================================================
// Rate Limiting & Backpressure
// ============================================================================

/// Token bucket rate limiter for RPC calls, simulations, and HTTP requests
#[derive(Debug)]
pub struct TokenBucket {
    tokens: Arc<RwLock<f64>>,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Arc<RwLock<Instant>>,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(capacity)),
            capacity,
            refill_rate,
            last_refill: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Try to consume tokens, returns true if successful
    pub async fn try_consume(&self, count: f64) -> bool {
        self.refill().await;
        let mut tokens = self.tokens.write().await;
        if *tokens >= count {
            *tokens -= count;
            true
        } else {
            false
        }
    }

    /// Wait until tokens are available, then consume
    pub async fn consume(&self, count: f64) {
        loop {
            if self.try_consume(count).await {
                return;
            }
            // Wait a bit before retrying
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn refill(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        if elapsed > 0.0 {
            let mut tokens = self.tokens.write().await;
            *tokens = (*tokens + elapsed * self.refill_rate).min(self.capacity);
            *last_refill = now;
        }
    }
}

/// Circuit breaker state for RPC endpoints
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Endpoint disabled
    HalfOpen,    // Testing if endpoint recovered
}

/// Task 2: Circuit breaker detailed status for monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerStatus {
    pub endpoint: String,
    pub state: CircuitState,
    pub failure_count: u32,
}

/// Circuit breaker for individual RPC endpoints
#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    failure_threshold: u32,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    timeout: Duration,
    half_open_success_threshold: u32,
    half_open_successes: Arc<AtomicU32>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            failure_threshold,
            last_failure_time: Arc::new(RwLock::new(None)),
            timeout,
            half_open_success_threshold: 2,
            half_open_successes: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Check if request is allowed
    pub async fn can_execute(&self) -> bool {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.timeout {
                        *state = CircuitState::HalfOpen;
                        self.half_open_successes.store(0, Ordering::Relaxed);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record successful execution
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::HalfOpen => {
                let successes = self.half_open_successes.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.half_open_success_threshold {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record failed execution
    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let mut last_failure = self.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
        
        if failures >= self.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Open;
            warn!("Circuit breaker opened after {} failures", failures);
        }
    }

    /// Get current state for monitoring
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Task 2: Get failure count for telemetry
    pub fn get_failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }
    
    /// Task 2: Manually trigger circuit open (for testing/admin control)
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        let mut last_failure = self.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
        warn!("Circuit breaker manually forced open");
    }
    
    /// Task 2: Manually reset circuit (for testing/admin control)
    pub async fn force_reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.half_open_successes.store(0, Ordering::Relaxed);
        info!("Circuit breaker manually reset to closed state");
    }
}

/// Retry policy with error classification
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 50,
            max_delay_ms: 2000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Classify error as retryable or fatal
    pub fn is_retryable(&self, error: &str) -> bool {
        // Retryable errors
        let retryable_patterns = [
            "timeout",
            "connection",
            "network",
            "temporarily unavailable",
            "too many requests",
            "rate limit",
            "503",
            "502",
            "504",
        ];
        
        let error_lower = error.to_lowercase();
        retryable_patterns.iter().any(|pattern| error_lower.contains(pattern))
    }
    
    /// Calculate delay for given attempt
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let delay_ms = (self.initial_delay_ms as f64 
            * self.backoff_multiplier.powi(attempt as i32)) as u64;
        Duration::from_millis(delay_ms.min(self.max_delay_ms))
    }
}

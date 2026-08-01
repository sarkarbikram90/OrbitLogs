// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Per-tenant token bucket rate limiting based on plan tier.

use dashmap::DashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Tenant Rate Limit Configuration per plan tier (requests per minute)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanTier {
    Developer,    // 100 req/min
    Professional, // 1,000 req/min
    Enterprise,   // 10,000 req/min
    Defense,      // Unlimited / 100,000 req/min
}

impl PlanTier {
    pub fn max_requests_per_minute(&self) -> f64 {
        match self {
            PlanTier::Developer => 100.0,
            PlanTier::Professional => 1000.0,
            PlanTier::Enterprise => 10000.0,
            PlanTier::Defense => 100000.0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "professional" => PlanTier::Professional,
            "enterprise" => PlanTier::Enterprise,
            "defense" => PlanTier::Defense,
            _ => PlanTier::Developer,
        }
    }
}

/// Thread-safe per-tenant Token Bucket Rate Limiter
pub struct TenantRateLimiter {
    // Key: tenant_id string -> Mutex<(current_tokens, last_refill_time)>
    buckets: DashMap<String, Mutex<(f64, Instant)>>,
}

impl Default for TenantRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantRateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
        }
    }

    /// Check if a request from a tenant is permitted.
    ///
    /// Returns `Ok(remaining_tokens)` if allowed, or `Err(retry_after_seconds)` if rate limit exceeded.
    pub fn check_rate_limit(&self, tenant_id: &str, plan_tier: PlanTier) -> Result<u32, u32> {
        let max_capacity = plan_tier.max_requests_per_minute();
        let refill_rate_per_sec = max_capacity / 60.0;
        let now = Instant::now();

        let entry = self
            .buckets
            .entry(tenant_id.to_string())
            .or_insert_with(|| Mutex::new((max_capacity, now)));

        let mut bucket = entry.lock().unwrap();
        let (ref mut tokens, ref mut last_refill) = *bucket;

        let elapsed_secs = now.duration_since(*last_refill).as_secs_f64();
        *tokens = (*tokens + elapsed_secs * refill_rate_per_sec).min(max_capacity);
        *last_refill = now;

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            Ok(*tokens as u32)
        } else {
            let missing_tokens = 1.0 - *tokens;
            let wait_secs = (missing_tokens / refill_rate_per_sec).ceil() as u32;
            Err(wait_secs.max(1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allowed() {
        let limiter = TenantRateLimiter::new();
        let res = limiter.check_rate_limit("tenant-1", PlanTier::Developer);
        assert!(res.is_ok());
    }

    #[test]
    fn test_rate_limiter_exhaustion() {
        let limiter = TenantRateLimiter::new();

        // Developer tier has capacity 100
        for _ in 0..100 {
            assert!(limiter
                .check_rate_limit("tenant-2", PlanTier::Developer)
                .is_ok());
        }

        // 101st request should be rejected
        let res = limiter.check_rate_limit("tenant-2", PlanTier::Developer);
        assert!(res.is_err());
    }
}

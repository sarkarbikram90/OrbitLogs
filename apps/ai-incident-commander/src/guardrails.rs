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
use regex::Regex;
use std::sync::OnceLock;

pub fn extract_satellite_id(query: &str, provided: Option<&str>) -> String {
    if let Some(id) = provided {
        if !id.trim().is_empty() {
            return id.trim().to_uppercase();
        }
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)\bSAT-\d{4}\b").unwrap());
    if let Some(mat) = re.find(query) {
        return mat.as_str().to_uppercase();
    }
    "SAT-0001".to_string()
}

pub fn sanitize_chat_input(query: &str) -> Result<(), String> {
    let lower = query.to_lowercase();

    // 1. Detect Prompt Injection
    // System prompt extraction attempts
    let injection_patterns = [
        "ignore previous",
        "system prompt",
        "system instruction",
        "jailbreak",
        "you must now",
        "bypass",
        "reveal your prompt",
        "act as a",
        "forget all",
    ];
    for pattern in &injection_patterns {
        if lower.contains(pattern) {
            return Err(
                "Input validation has failed: Potential prompt injection attempt detected."
                    .to_string(),
            );
        }
    }

    // 2. Detect SQL Injection attempts using regex to prevent false positives on '--'
    // in satellite names or hyphenated words
    static SQL_RE: OnceLock<Regex> = OnceLock::new();
    let re = SQL_RE.get_or_init(|| {
        Regex::new(r"(?i)(union\s+select|drop\s+table|delete\s+from|insert\s+into|select\s+\*|or\s+1\s*=\s*1|or\s+'1'\s*=\s*'1'|--\s)").unwrap()
    });

    if re.is_match(&lower) {
        return Err(
            "Input validation has failed: Potential database injection attempt detected."
                .to_string(),
        );
    }

    Ok(())
}

pub fn sanitize_chat_output(response: &str) -> String {
    let lower = response.to_lowercase();

    // 1. Check for leakage of sensitive keys or connections
    if lower.contains("postgres://")
        || lower.contains("postgresql://")
        || lower.contains("api_key")
        || lower.contains("jwt_secret")
    {
        tracing::warn!(
            "Guardrail Warning: LLM output blocked due to sensitive system data leakage detection."
        );
        return "System Warning: Response blocked by LLM output safety guardrails (leakage protection)."
            .to_string();
    }

    // 2. Check for leak of system/internal instructions
    if lower.contains("system instructions:") || lower.contains("you are antigravity") {
        tracing::warn!(
            "Guardrail Warning: LLM output blocked due to system instruction leak detection."
        );
        return "System Warning: Response blocked by LLM output safety guardrails (instruction protection)."
            .to_string();
    }

    response.to_string()
}

use std::{env, process::Command};

const SEARXNG_FALLBACK_HEALTH_ENDPOINT: &str = "http://127.0.0.1:8080";
const SILICONFLOW_ENDPOINT: &str = "https://api.siliconflow.cn/v1/rerank";

fn main() {
    let searxng_url = check_required_env("SEARXNG_URL");
    let siliconflow_api_key = check_required_env("SILICONFLOW_API_KEY");

    // 可选网络检查：仅告警，不阻断构建
    check_searxng_connectivity(searxng_url.as_deref());
    check_siliconflow_connectivity(siliconflow_api_key.as_deref());
}

fn check_required_env(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                println!("cargo:warning={} environment variable is set but empty", key);
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(_) => {
            println!("cargo:warning={} environment variable is not set", key);
            None
        }
    }
}

fn check_searxng_connectivity(url: Option<&str>) {
    let target = url.unwrap_or(SEARXNG_FALLBACK_HEALTH_ENDPOINT);
    match curl_status_code(target, None) {
        Ok(code) => {
            if code == "000" {
                println!(
                    "cargo:warning=SearxNG endpoint is unreachable ({}), curl returned status code 000",
                    target
                );
            }
        }
        Err(reason) => {
            println!(
                "cargo:warning=Failed to check SearxNG endpoint ({}): {}",
                target, reason
            );
        }
    }
}

fn check_siliconflow_connectivity(api_key: Option<&str>) {
    let auth_header = api_key.map(|k| format!("Authorization: Bearer {}", k));
    match curl_status_code(SILICONFLOW_ENDPOINT, auth_header.as_deref()) {
        Ok(code) => {
            if code == "000" {
                println!(
                    "cargo:warning=SiliconFlow endpoint is unreachable ({}), curl returned status code 000",
                    SILICONFLOW_ENDPOINT
                );
            }
        }
        Err(reason) => {
            println!(
                "cargo:warning=Failed to check SiliconFlow endpoint ({}): {}",
                SILICONFLOW_ENDPOINT, reason
            );
        }
    }
}

fn curl_status_code(url: &str, auth_header: Option<&str>) -> Result<String, String> {
    let mut command = Command::new("curl");
    command
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("5")
        .arg("--output")
        .arg("/dev/null")
        .arg("--write-out")
        .arg("%{http_code}");

    if let Some(header) = auth_header {
        command.arg("--header").arg(header);
    }

    let output = command.arg(url).output().map_err(|err| {
        format!(
            "unable to execute curl (is it installed and in PATH?): {}",
            err
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl exited with status {:?}: {}", output.status.code(), stderr.trim()));
    }

    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if code.is_empty() {
        return Err("curl produced empty status code".to_string());
    }

    Ok(code)
}

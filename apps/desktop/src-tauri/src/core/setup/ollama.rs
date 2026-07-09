//! Ollama setup automation: detection, install, server lifecycle, model
//! pulls, and hardware-aware model recommendations.
//!
//! Everything here is plain functions — no `#[tauri::command]`s. The
//! orchestrator wraps these for the frontend separately.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const SERVER_START_POLL_TIMEOUT: Duration = Duration::from_secs(10);
const SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(300);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Snapshot of local Ollama install/runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub server_running: bool,
    pub version: Option<String>,
    pub models: Vec<String>,
}

/// A single progress update while pulling a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    pub model: String,
    pub status: String,
    pub completed: u64,
    pub total: u64,
    pub percent: f32,
}

/// Locate the `ollama` executable: first the well-known winget install path
/// under `%LOCALAPPDATA%\Programs\Ollama\ollama.exe`, then fall back to
/// whatever `ollama` resolves to on PATH (via `where`).
pub fn ollama_exe_path() -> Option<PathBuf> {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let candidate = PathBuf::from(local_app_data)
            .join("Programs")
            .join("Ollama")
            .join("ollama.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let output = Command::new("where").arg("ollama").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }
    let path = PathBuf::from(first_line);
    path.is_file().then_some(path)
}

#[derive(Debug, Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Debug, Deserialize)]
struct TagModel {
    name: String,
}

/// Probe the local machine and a (possibly running) Ollama server at
/// `base_url`. Never returns an error: a down server simply yields
/// `server_running: false`.
pub async fn detect(base_url: &str) -> OllamaStatus {
    let installed = ollama_exe_path().is_some();

    let client = match reqwest::Client::builder()
        .timeout(DEFAULT_PROBE_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return OllamaStatus {
                installed,
                server_running: false,
                version: None,
                models: Vec::new(),
            }
        }
    };

    let version_url = format!("{base_url}/api/version");
    let version = match client.get(&version_url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<VersionResponse>().await.ok(),
        _ => None,
    };

    let Some(version) = version else {
        return OllamaStatus {
            installed,
            server_running: false,
            version: None,
            models: Vec::new(),
        };
    };

    let tags_url = format!("{base_url}/api/tags");
    let models = match client.get(&tags_url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<TagsResponse>()
            .await
            .map(|tags| tags.models.into_iter().map(|m| m.name).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    OllamaStatus {
        installed,
        server_running: true,
        version: Some(version.version),
        models,
    }
}

/// Install Ollama via winget. Long-running and side-effecting; not exercised
/// by automated tests.
pub fn install_via_winget() -> anyhow::Result<()> {
    let output = Command::new("winget")
        .args([
            "install",
            "--id",
            "Ollama.Ollama",
            "-e",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("winget install failed (status {}): {stderr}", output.status);
    }
}

/// Ensure the Ollama server is running at `base_url`. If installed but not
/// running, spawns `ollama serve` detached (no visible console window) and
/// polls until it responds or the timeout elapses. Side-effecting; not
/// exercised by automated tests.
///
/// This function is synchronous (spawns its own single-threaded Tokio
/// runtime internally for the HTTP probes) so callers outside an async
/// context can use it directly; if called from within an existing Tokio
/// runtime, run it via `spawn_blocking` to avoid blocking that runtime's
/// worker thread.
pub fn ensure_server_running(base_url: &str) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let status = rt.block_on(detect(base_url));
    if !status.installed {
        anyhow::bail!("ollama is not installed");
    }
    if status.server_running {
        return Ok(());
    }

    let exe = ollama_exe_path().ok_or_else(|| anyhow::anyhow!("ollama executable not found"))?;

    let mut cmd = Command::new(exe);
    cmd.arg("serve");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn()?;

    let deadline = std::time::Instant::now() + SERVER_START_POLL_TIMEOUT;
    while std::time::Instant::now() < deadline {
        let status = rt.block_on(detect(base_url));
        if status.server_running {
            return Ok(());
        }
        std::thread::sleep(SERVER_START_POLL_INTERVAL);
    }

    anyhow::bail!("ollama serve did not become ready within {SERVER_START_POLL_TIMEOUT:?}")
}

#[derive(Debug, Deserialize)]
struct PullLine {
    #[serde(default)]
    status: String,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    completed: Option<u64>,
    #[serde(default)]
    error: Option<String>,
}

/// Pull `model` from `base_url`, invoking `on_progress` for each streamed
/// NDJSON status update. Resolves when the stream ends or a line reports
/// `status: "success"`. Any `{"error": "..."}` line surfaces as `Err`.
pub async fn pull_model<F: FnMut(PullProgress)>(
    base_url: &str,
    model: &str,
    mut on_progress: F,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{base_url}/api/pull");
    let body = json!({ "model": model, "stream": true });

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let mut byte_stream = resp
        .bytes_stream()
        .map(|chunk| chunk.map(|b| String::from_utf8_lossy(&b).into_owned()));

    let mut buffer = String::new();

    macro_rules! process_line {
        ($line:expr) => {{
            let line = $line.trim();
            if !line.is_empty() {
                let parsed: PullLine = serde_json::from_str(line)?;
                if let Some(err) = parsed.error {
                    anyhow::bail!("ollama pull error: {err}");
                }
                let total = parsed.total.unwrap_or(0);
                let completed = parsed.completed.unwrap_or(0);
                let percent = if total == 0 {
                    0.0
                } else {
                    (completed as f32 / total as f32) * 100.0
                };
                let is_success = parsed.status == "success";
                on_progress(PullProgress {
                    model: model.to_string(),
                    status: parsed.status,
                    completed,
                    total,
                    percent,
                });
                if is_success {
                    return Ok(());
                }
            }
        }};
    }

    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&chunk);
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            process_line!(line);
        }
    }

    if !buffer.trim().is_empty() {
        let remainder = std::mem::take(&mut buffer);
        process_line!(remainder);
    }

    Ok(())
}

/// Best-effort total physical RAM in GB. Falls back to a conservative `8`
/// on any failure to query the OS.
pub fn total_ram_gb() -> u32 {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
        ])
        .output();

    let Ok(output) = output else {
        return 8;
    };
    if !output.status.success() {
        return 8;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytes: u64 = match stdout.trim().parse() {
        Ok(bytes) => bytes,
        Err(_) => return 8,
    };

    let gb = bytes / (1024 * 1024 * 1024);
    if gb == 0 {
        8
    } else {
        gb as u32
    }
}

/// Recommended models to pull given the machine's total RAM: the policy
/// chat model (mirrors `provider::pick_default_model`'s RAM tiering) plus
/// the embedding model.
pub fn recommended_models(ram_gb: u32) -> Vec<String> {
    let chat_model = if ram_gb >= 24 { "qwen3:8b" } else { "qwen3:4b" };
    vec![chat_model.to_string(), "nomic-embed-text".to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn detect_reports_running_server_with_models() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version": "0.5.1"})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{"name": "qwen3:4b"}, {"name": "nomic-embed-text"}]
            })))
            .mount(&server)
            .await;

        let status = detect(&server.uri()).await;
        assert!(status.server_running);
        assert_eq!(status.version, Some("0.5.1".to_string()));
        assert_eq!(
            status.models,
            vec!["qwen3:4b".to_string(), "nomic-embed-text".to_string()]
        );
    }

    #[tokio::test]
    async fn detect_handles_down_server_without_panicking() {
        // Nothing listening on this port.
        let status = detect("http://127.0.0.1:1").await;
        assert!(!status.server_running);
        assert_eq!(status.version, None);
        assert!(status.models.is_empty());
    }

    async fn progress_from_ndjson(body: &str) -> Vec<PullProgress> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/pull"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(body.as_bytes().to_vec(), "application/x-ndjson"),
            )
            .mount(&server)
            .await;

        let mut progress = Vec::new();
        pull_model(&server.uri(), "qwen3:4b", |p| progress.push(p))
            .await
            .unwrap();
        progress
    }

    #[tokio::test]
    async fn pull_model_reports_progress_and_success() {
        let body = concat!(
            "{\"status\":\"pulling\",\"total\":100,\"completed\":25}\n",
            "{\"status\":\"pulling\",\"total\":100,\"completed\":75}\n",
            "{\"status\":\"success\"}\n",
        );
        let progress = progress_from_ndjson(body).await;
        assert_eq!(progress.len(), 3);
        assert_eq!(progress[0].percent, 25.0);
        assert_eq!(progress[1].percent, 75.0);
        assert_eq!(progress[2].status, "success");
        assert_eq!(progress[2].percent, 0.0);
        assert!(progress.iter().all(|p| p.model == "qwen3:4b"));
    }

    #[tokio::test]
    async fn pull_model_handles_two_objects_in_one_chunk_and_surfaces_errors() {
        let body = concat!(
            "{\"status\":\"pulling\",\"total\":10,\"completed\":5}\n",
            "{\"status\":\"pulling\",\"total\":10,\"completed\":10}\n",
        );
        let progress = progress_from_ndjson(body).await;
        assert_eq!(progress.len(), 2);
        assert_eq!(progress[0].percent, 50.0);
        assert_eq!(progress[1].percent, 100.0);

        // Error line.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/pull"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                b"{\"error\":\"model not found\"}\n".to_vec(),
                "application/x-ndjson",
            ))
            .mount(&server)
            .await;
        let result = pull_model(&server.uri(), "bogus", |_| {}).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("model not found"));
    }

    #[test]
    fn pull_model_line_parser_handles_split_mid_json() {
        // Mirrors the provider's line-buffer robustness test: drive the
        // same split-across-chunks buffer logic used inside pull_model by
        // reimplementing the drain loop against a manual buffer.
        let mut buffer = String::new();
        let mut lines = Vec::new();
        buffer.push_str("{\"status\":\"pulling\",\"tot");
        while let Some(pos) = buffer.find('\n') {
            lines.push(buffer.drain(..=pos).collect::<String>());
        }
        assert!(lines.is_empty(), "no complete line yet");

        buffer.push_str("al\":10,\"completed\":10}\n{\"status\":\"success\"}\n");
        while let Some(pos) = buffer.find('\n') {
            lines.push(buffer.drain(..=pos).collect::<String>());
        }
        assert_eq!(lines.len(), 2);
        let parsed0: PullLine = serde_json::from_str(lines[0].trim()).unwrap();
        let parsed1: PullLine = serde_json::from_str(lines[1].trim()).unwrap();
        assert_eq!(parsed0.completed, Some(10));
        assert_eq!(parsed1.status, "success");
    }

    #[test]
    fn recommended_models_table() {
        let hi = recommended_models(32);
        assert_eq!(hi[0], "qwen3:8b");
        assert!(hi.contains(&"nomic-embed-text".to_string()));

        let lo = recommended_models(16);
        assert_eq!(lo[0], "qwen3:4b");
        assert!(lo.contains(&"nomic-embed-text".to_string()));
    }

    #[test]
    fn total_ram_gb_returns_positive_value() {
        assert!(total_ram_gb() > 0);
    }
}

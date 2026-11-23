use crate::{
    prompts::ChatMessage,
    types::{AppSettings, ComputeDevice, EngineStatus},
    Error, Result,
};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
mod generation;
pub const CONTEXT_TOKENS: usize = 16_384;

pub const MODEL_NAME: &str = "Qwen3.5-9B · Q4_K_M";
#[derive(Clone)]
pub struct ModelClient {
    client: Client,
    endpoint: String,
    token: String,
}

pub struct Engine {
    child: Option<Child>,
    #[cfg(windows)]
    job: Option<crate::windows_job::Job>,
    connection: Option<ModelClient>,
    resources: PathBuf,
    requested_device: ComputeDevice,
    backend: Option<String>,
    device_name: Option<String>,
    fallback_reason: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetLock {
    schema_version: u32,
    model: AssetFile,
    projector: AssetFile,
    files: Vec<AssetFile>,
}
#[derive(Deserialize)]
struct AssetFile {
    path: String,
    size: u64,
    sha256: String,
}
struct RuntimePaths {
    cpu: PathBuf,
    gpu: PathBuf,
    model: PathBuf,
    projector: PathBuf,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct GpuDevice {
    id: String,
    name: String,
}

impl Engine {
    pub fn new(resources: PathBuf) -> Self {
        Self {
            child: None,
            #[cfg(windows)]
            job: None,
            connection: None,
            resources,
            requested_device: ComputeDevice::Auto,
            backend: None,
            device_name: None,
            fallback_reason: None,
        }
    }
    pub fn requested_device(&self) -> ComputeDevice {
        self.requested_device
    }
    pub fn status(&mut self) -> EngineStatus {
        let alive = self
            .child
            .as_mut()
            .is_some_and(|c| matches!(c.try_wait(), Ok(None)));
        if !alive {
            self.connection = None;
            self.backend = None;
            self.device_name = None;
        }
        let (state, detail) = if alive && self.connection.is_some() {
            let backend = if self.backend.as_deref() == Some("vulkan") {
                "GPU · Vulkan"
            } else {
                "CPU"
            };
            (
                "ready",
                format!(
                    "{backend} · {}",
                    self.device_name.as_deref().unwrap_or("Yerel model")
                ),
            )
        } else if !self.resources.join("asset-lock.json").exists() {
            (
                "missing",
                "Model paketi henüz hazırlanmadı. Geliştirme için pnpm assets:fetch çalıştırın."
                    .into(),
            )
        } else {
            ("stopped", "Yerel model hazırlanmayı bekliyor.".into())
        };
        EngineStatus {
            state: state.into(),
            detail,
            model: MODEL_NAME.into(),
            requested_device: self.requested_device,
            backend: self.backend.clone(),
            device_name: self.device_name.clone(),
            fallback_reason: self.fallback_reason.clone(),
        }
    }
    pub async fn start(&mut self) -> Result<ModelClient> {
        self.start_with_device(ComputeDevice::Auto).await
    }
    pub async fn start_with_device(&mut self, preference: ComputeDevice) -> Result<ModelClient> {
        if self.status().state == "ready" && self.requested_device == preference {
            return Ok(self.connection.clone().unwrap());
        }
        self.stop();
        self.requested_device = preference;
        self.fallback_reason = None;
        let directory = self.resources.clone();
        let paths = tokio::task::spawn_blocking(move || verify_resources(&directory))
            .await
            .map_err(|_| Error::Model("Paket denetimi tamamlanamadı.".into()))??;
        if preference != ComputeDevice::Cpu {
            let detected = probe_gpu(&paths.gpu).await;
            match detected {
                Ok(device) => match self
                    .launch(&paths.gpu, &paths.model, &paths.projector, Some(&device))
                    .await
                {
                    Ok(client) => return Ok(client),
                    Err(error) if preference == ComputeDevice::Auto => {
                        self.stop();
                        self.fallback_reason = Some(format!(
                            "GPU başlatılamadığı için CPU kullanılıyor: {error}"
                        ));
                    }
                    Err(error) => {
                        self.stop();
                        return Err(error);
                    }
                },
                Err(error) if preference == ComputeDevice::Auto => {
                    self.fallback_reason =
                        Some(format!("GPU kullanılamadığı için CPU seçildi: {error}"));
                }
                Err(error) => return Err(error),
            }
        }
        self.launch(&paths.cpu, &paths.model, &paths.projector, None)
            .await
    }
    async fn launch(
        &mut self,
        executable: &Path,
        model: &Path,
        projector: &Path,
        gpu: Option<&GpuDevice>,
    ) -> Result<ModelClient> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let client = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(3))
            .read_timeout(Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Model("Yerel bağlantı kurulamadı.".into()))?;
        let connection = ModelClient {
            client,
            endpoint: format!("http://127.0.0.1:{port}"),
            token: token.clone(),
        };
        let mut command = runtime_command(executable);
        command
            .env("LLAMA_API_KEY", token)
            .arg("--model")
            .arg(model)
            .arg("--mmproj")
            .arg(projector)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--ctx-size",
                "16384",
                "--image-max-tokens",
                "1536",
                "--parallel",
                "1",
                "--device",
                gpu.map(|g| g.id.as_str()).unwrap_or("none"),
                "--n-gpu-layers",
                if gpu.is_some() { "auto" } else { "0" },
                "--fit",
                "on",
                "--fit-target",
                "1536",
                "--fit-ctx",
                "16384",
                "--jinja",
                "--no-webui",
                "--alias",
                "aevra",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if gpu.is_none() {
            command.arg("--no-mmproj-offload");
        }
        self.backend = Some(if gpu.is_some() { "vulkan" } else { "cpu" }.into());
        self.device_name = Some(
            gpu.map(|g| g.name.clone())
                .unwrap_or_else(|| "İşlemci".into()),
        );
        drop(listener);
        self.child = Some(command.spawn().map_err(|_| Error::Model("Model motoru başlatılamadı. Çalışma dosyalarını ve ekran kartı sürücüsünü kontrol edin.".into()))?);
        #[cfg(windows)]
        {
            let assigned = crate::windows_job::Job::new().and_then(|job| {
                job.assign(self.child.as_ref().unwrap())?;
                Ok(job)
            });
            match assigned {
                Ok(job) => self.job = Some(job),
                Err(error) => {
                    self.stop();
                    return Err(error.into());
                }
            }
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        while std::time::Instant::now() < deadline {
            if self.child.as_mut().unwrap().try_wait()?.is_some() {
                self.stop();
                return Err(Error::Model(if gpu.is_some() { "GPU motoru modeli yükleyemedi. Ekran kartı belleği veya sürücü uyumluluğunu kontrol edin." } else { "CPU motoru modeli yükleyemedi. Belleği ve paket dosyalarını kontrol edin." }.into()));
            }
            if let Ok(response) = connection
                .client
                .get(format!("{}/health", connection.endpoint))
                .bearer_auth(&connection.token)
                .timeout(Duration::from_secs(1))
                .send()
                .await
            {
                if response.status().is_success() {
                    self.connection = Some(connection.clone());
                    return Ok(connection);
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        self.stop();
        Err(Error::Model(
            "Model 120 saniye içinde hazırlanamadı.".into(),
        ))
    }
    pub fn stop(&mut self) {
        self.connection = None;
        self.backend = None;
        self.device_name = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(windows)]
        {
            self.job = None;
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

fn runtime_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .current_dir(executable.parent().unwrap())
        .env_clear()
        .stdin(Stdio::null());
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
async fn probe_gpu(executable: &Path) -> Result<GpuDevice> {
    #[cfg(windows)]
    {
        let bundled_loader = executable.parent().unwrap().join("vulkan-1.dll").is_file();
        let system_loader = std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32/vulkan-1.dll").is_file())
            .unwrap_or(false);
        if !bundled_loader && !system_loader {
            return Err(Error::Model("Vulkan sürücüsü bulunamadı. CPU kullanılabilir; GPU için uyumlu ekran kartı sürücüsü gerekir.".into()));
        }
    }
    let mut command = tokio::process::Command::from(runtime_command(executable));
    command
        .arg("--list-devices")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = tokio::time::timeout(Duration::from_secs(10), command.output())
        .await
        .map_err(|_| Error::Model("Ekran kartı denetimi zaman aşımına uğradı.".into()))?
        .map_err(|_| {
            Error::Model("GPU motoru çalıştırılamadı. Uyumlu ekran kartı sürücüsü gerekli.".into())
        })?;
    if !output.status.success() {
        return Err(Error::Model(
            "GPU denetimi başarısız. Ekran kartı sürücüsünü kontrol edin.".into(),
        ));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_gpu_devices(&text)
        .into_iter()
        .max_by_key(|d| {
            let name = d.name.to_ascii_lowercase();
            if name.contains("nvidia") {
                3
            } else if name.contains("radeon") || name.contains("amd") {
                2
            } else {
                1
            }
        })
        .ok_or_else(|| {
            Error::Model(
                "Uyumlu Vulkan ekran kartı bulunamadı. CPU veya Otomatik seçebilirsiniz.".into(),
            )
        })
}
fn parse_gpu_devices(text: &str) -> Vec<GpuDevice> {
    text.lines()
        .filter_map(|line| {
            let (id, label) = line.trim().split_once(':')?;
            let index = id.strip_prefix("Vulkan")?;
            if index.is_empty() || !index.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let name = label.trim().split(" (").next()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(GpuDevice {
                id: id.into(),
                name: name.chars().take(180).collect(),
            })
        })
        .collect()
}
fn verify_resources(directory: &Path) -> Result<RuntimePaths> {
    let bytes = std::fs::read(directory.join("asset-lock.json")).map_err(|_| {
        Error::Model("Model paketi eksik. Önce pnpm assets:fetch çalıştırın.".into())
    })?;
    let manifest: AssetLock = serde_json::from_slice(&bytes)?;
    if manifest.schema_version != 1 {
        return Err(Error::Model("Paket sürümü desteklenmiyor.".into()));
    }
    for required in [
        "runtime/llama-server.exe",
        "runtime/vulkan/llama-server.exe",
        "runtime/vulkan/ggml-vulkan.dll",
    ] {
        if !manifest.files.iter().any(|f| f.path == required) {
            return Err(Error::Model(format!(
                "Paket dosyası eksik: {required}. pnpm assets:fetch çalıştırın."
            )));
        }
    }
    for file in manifest
        .files
        .iter()
        .chain(std::iter::once(&manifest.model))
        .chain(std::iter::once(&manifest.projector))
    {
        verify_asset(directory, file)?;
    }
    Ok(RuntimePaths {
        cpu: directory.join("runtime/llama-server.exe"),
        gpu: directory.join("runtime/vulkan/llama-server.exe"),
        model: directory.join(manifest.model.path),
        projector: directory.join(manifest.projector.path),
    })
}

pub fn verify_bundled_file(directory: &Path, relative: &str) -> Result<()> {
    let manifest: AssetLock =
        serde_json::from_slice(&std::fs::read(directory.join("asset-lock.json"))?)?;
    let file = manifest
        .files
        .iter()
        .find(|f| f.path == relative)
        .ok_or_else(|| Error::Validation("Gerekli paket dosyası bulunamadı.".into()))?;
    verify_asset(directory, file)
}
fn verify_asset(directory: &Path, file: &AssetFile) -> Result<()> {
    if file.path.contains('\\')
        || Path::new(&file.path)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::Model("Geçersiz paket dosyası yolu.".into()));
    }
    let mut reader = File::open(directory.join(&file.path))?;
    if reader.metadata()?.len() != file.size {
        return Err(Error::Model("Paket dosyası boyutu eşleşmiyor.".into()));
    }
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != file.sha256 {
        return Err(Error::Model("Paket bütünlük denetimi başarısız.".into()));
    }
    Ok(())
}

impl ModelClient {
    pub async fn generate(
        &self,
        messages: Vec<ChatMessage>,
        cancel: CancellationToken,
        delta: impl FnMut(String),
    ) -> Result<String> {
        self.generate_with_settings(messages, cancel, &AppSettings::default(), delta)
            .await
    }

    async fn generate_segment(
        &self,
        messages: Vec<ChatMessage>,
        cancel: CancellationToken,
        settings: &AppSettings,
        mut delta: impl FnMut(String),
    ) -> Result<Segment> {
        settings.validate()?;
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            result = self.client.post(format!("{}/v1/chat/completions", self.endpoint))
                .bearer_auth(&self.token).json(&serde_json::json!({
                    "model": "aevra", "messages": messages.iter().map(ChatMessage::as_json).collect::<Vec<_>>(), "stream": true,
                    "max_tokens": settings.max_tokens, "temperature": settings.temperature, "top_p": 0.8, "top_k": 20,
                    "presence_penalty": 1.5, "chat_template_kwargs": { "enable_thinking": false }
                })).send() => result.map_err(|_| Error::Model("Yerel modele bağlanılamadı.".into()))?
        };
        if !response.status().is_success() {
            return Err(Error::Model(format!(
                "Model isteği reddetti ({}). Metni kısaltmayı deneyin.",
                response.status()
            )));
        }
        let mut stream = response.bytes_stream();
        let mut parser = SseParser::default();
        let mut output = String::new();
        loop {
            let chunk = tokio::select! {
                _ = cancel.cancelled() => return Err(Error::Cancelled),
                chunk = stream.next() => chunk,
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk.map_err(|_| {
                Error::Model("Model bağlantısı kesildi. Konuşma geçmişi korundu.".into())
            })?;
            for text in parser.push(&chunk)? {
                output.push_str(&text);
                if output.len() > 512_000 {
                    return Err(Error::Model("Model yanıt sınırını aştı.".into()));
                }
                delta(text);
            }
            if parser.done {
                break;
            }
        }
        if !parser.done || output.trim().is_empty() {
            return Err(Error::Model(
                "Model tam bir yanıt üretmedi. Tekrar deneyin.".into(),
            ));
        }
        Ok(Segment {
            text: output,
            finish_reason: parser.finish_reason,
        })
    }
}

#[derive(Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    pub done: bool,
    pub finish_reason: Option<String>,
}

impl SseParser {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>> {
        self.buffer.extend_from_slice(bytes);
        if self.buffer.len() > 256_000 {
            return Err(Error::Model("Geçersiz model akışı.".into()));
        }
        let mut parts = Vec::new();
        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            let line = self.buffer.drain(..=end).collect::<Vec<_>>();
            let line = std::str::from_utf8(&line)
                .map_err(|_| Error::Model("Geçersiz UTF-8 yanıtı.".into()))?
                .trim();
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                self.done = true;
                break;
            }
            let value: serde_json::Value = serde_json::from_str(data)?;
            if value.get("error").is_some() {
                return Err(Error::Model("Model üretim sırasında hata bildirdi.".into()));
            }
            if let Some(text) = value["choices"][0]["delta"]["content"].as_str() {
                parts.push(text.to_string());
            }
            if let Some(reason) = value["choices"][0]["finish_reason"].as_str() {
                self.finish_reason = Some(reason.into());
            }
        }
        Ok(parts)
    }
}
struct Segment {
    text: String,
    finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gpu_probe_ignores_logs_and_preserves_device_ids() {
        let parsed = parse_gpu_devices("Available devices:\n Vulkan0: NVIDIA GeForce RTX 3070 Ti Laptop GPU (8017 MiB, 7249 MiB free)\n Vulkan1: Intel(R) Iris(R) Xe Graphics (8062 MiB)\n ggml_vulkan: found 2 devices\n Vulkanbad: fake\n CPU: test");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "Vulkan0");
        assert_eq!(parsed[0].name, "NVIDIA GeForce RTX 3070 Ti Laptop GPU");
        assert!(parse_gpu_devices("No devices available").is_empty());
    }
    #[test]
    fn stream_handles_every_byte_boundary_including_turkish_and_done() {
        let input =
            "data: {\"choices\":[{\"delta\":{\"content\":\"Türkçe 🧠\"}}]}\r\n\r\ndata: [DONE]\n\n";
        for split in 0..input.len() {
            let mut parser = SseParser::default();
            let mut parts = parser.push(&input.as_bytes()[..split]).unwrap();
            parts.extend(parser.push(&input.as_bytes()[split..]).unwrap());
            assert_eq!(parts.join(""), "Türkçe 🧠");
            assert!(parser.done);
        }
    }
    #[test]
    fn malformed_stream_is_rejected() {
        assert!(SseParser::default().push(b"data: broken\n").is_err());
        assert!(SseParser::default().push(&vec![0; 256_001]).is_err());
    }
}

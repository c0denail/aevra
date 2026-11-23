use super::*;

impl ModelClient {
    /// One schema-constrained local classification; truncated output is never accepted.
    pub async fn generate_json(
        &self,
        messages: Vec<ChatMessage>,
        schema: serde_json::Value,
        cancel: CancellationToken,
    ) -> Result<String> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let policy = AppSettings {
            max_tokens: 768,
            temperature: 0.0,
            ..AppSettings::default()
        };
        if !self.fits(&messages, &policy, &cancel).await? {
            return Err(Error::Validation(
                "Hafıza girdisi bağlam sınırını aşıyor.".into(),
            ));
        }
        let response = tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),r=self.client.post(format!("{}/v1/chat/completions",self.endpoint)).bearer_auth(&self.token).json(&serde_json::json!({"model":"aevra","messages":messages.iter().map(ChatMessage::as_json).collect::<Vec<_>>(),"stream":false,"max_tokens":768,"temperature":0.0,"presence_penalty":0.0,"response_format":{"type":"json_schema","json_schema":{"name":"personal_memory","strict":true,"schema":schema}},"chat_template_kwargs":{"enable_thinking":false}})).send()=>r.map_err(|_|Error::Model("Yerel hafıza bağlantısı kurulamadı.".into()))?};
        if !response.status().is_success() {
            return Err(Error::Model(
                "Yerel motor hafıza şemasını kabul etmedi.".into(),
            ));
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        loop {
            let chunk = tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),c=stream.next()=>c};
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk.map_err(|_| Error::Model("Hafıza bağlantısı kesildi.".into()))?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() > 64000 {
                return Err(Error::Validation("Hafıza yanıt sınırı aşıldı.".into()));
            }
        }
        let data: serde_json::Value = serde_json::from_slice(&bytes)?;
        if data["choices"][0]["finish_reason"] != "stop" {
            return Err(Error::Validation("Hafıza yanıtı tamamlanmadı.".into()));
        }
        data["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| Error::Validation("Hafıza yanıtı bulunamadı.".into()))
    }
    pub async fn generate_with_settings(
        &self,
        messages: Vec<ChatMessage>,
        cancel: CancellationToken,
        settings: &AppSettings,
        delta: impl FnMut(String),
    ) -> Result<String> {
        self.generate_with_progress(messages, cancel, settings, delta, |_| {})
            .await
    }

    /// A token budget limits one generation segment, not whether a response is complete.
    pub async fn generate_with_progress(
        &self,
        mut messages: Vec<ChatMessage>,
        cancel: CancellationToken,
        settings: &AppSettings,
        mut delta: impl FnMut(String),
        mut status: impl FnMut(String),
    ) -> Result<String> {
        settings.validate()?;
        let adjusted = AppSettings {
            max_tokens: settings
                .max_tokens
                .min(if messages.iter().any(|m| !m.images.is_empty()) {
                    2048
                } else {
                    8192
                }),
            ..settings.clone()
        };
        let settings = &adjusted;
        if messages.len() < 2 {
            return Err(Error::Validation("Model isteği boş.".into()));
        }
        while !self.fits(&messages, settings, &cancel).await? {
            if messages.len() <= 2 {
                return Err(Error::Validation(
                    "İçerik modelin bağlamına sığmıyor. Metni veya ekleri küçültün.".into(),
                ));
            }
            // Remove the oldest historical pair, never the system or current request.
            let remove = if messages.len() >= 4 { 2 } else { 1 };
            messages.drain(1..1 + remove);
        }
        let base = vec![messages[0].clone(), messages.last().unwrap().clone()];
        let mut output = String::new();
        for round in 0..16 {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            if round > 0 {
                status("Yanıt tamamlanıyor…".into());
                messages = base.clone();
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: output.clone(),
                    images: vec![],
                });
                messages.push(ChatMessage{role:"user".into(),content:"Continue the unfinished answer exactly from where it was cut off. Keep the same language, numbering and code block. Do not repeat previous content or add an introduction. Complete the original task and finish naturally.".into(),images:vec![]});
                while !self.fits(&messages, settings, &cancel).await? {
                    let content = &mut messages[2].content;
                    let count = content.chars().count();
                    if count <= 512 {
                        return Err(Error::OutputLimit);
                    }
                    *content = content.chars().skip(count / 4).collect();
                }
            }
            let segment = self
                .generate_segment(messages.clone(), cancel.clone(), settings, |text| {
                    delta(text)
                })
                .await?;
            output.push_str(&segment.text);
            if output.len() > 512_000 {
                return Err(Error::OutputLimit);
            }
            match segment.finish_reason.as_deref() {
                Some("stop") => return Ok(output),
                Some("length") => (),
                _ => {
                    return Err(Error::Model(
                        "Model yanıtı tamamlandığını doğrulamadı. Yazılan bölüm kaydedildi.".into(),
                    ))
                }
            }
        }
        Err(Error::OutputLimit)
    }

    async fn fits(
        &self,
        messages: &[ChatMessage],
        settings: &AppSettings,
        cancel: &CancellationToken,
    ) -> Result<bool> {
        let text = messages
            .iter()
            .map(|m| format!("{}: {}\n", m.role, m.content))
            .collect::<String>();
        let result = tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),result=self.client.post(format!("{}/tokenize",self.endpoint)).bearer_auth(&self.token).json(&serde_json::json!({"content":text,"add_special":true})).timeout(Duration::from_secs(30)).send()=>result.map_err(|_|Error::Model("Bağlam boyutu kontrol edilemedi.".into()))?};
        if !result.status().is_success() {
            return Err(Error::Model("Model bağlam sayımı başarısız.".into()));
        }
        let body: serde_json::Value = tokio::select! {_ = cancel.cancelled()=>return Err(Error::Cancelled),body=result.json()=>body.map_err(|_|Error::Model("Model bağlam sayımı okunamadı.".into()))?};
        let tokens = body["tokens"]
            .as_array()
            .ok_or_else(|| Error::Model("Geçersiz token sayımı.".into()))?
            .len();
        // Reserve template overhead and image embeddings in addition to actual text tokens.
        let images = messages.iter().map(|m| m.images.len()).sum::<usize>();
        Ok(tokens + images * 1664 + 512 + settings.max_tokens as usize <= CONTEXT_TOKENS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    async fn server(
        replies: Vec<(&'static str, Option<&'static str>)>,
    ) -> (
        ModelClient,
        Arc<Mutex<Vec<serde_json::Value>>>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let mut replies = VecDeque::from(replies);
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut data = Vec::new();
                let split = loop {
                    let mut chunk = [0; 4096];
                    let n = socket.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    data.extend(&chunk[..n]);
                    if let Some(i) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&data[..split]);
                let size = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while data.len() < split + size {
                    let mut chunk = [0; 4096];
                    let n = socket.read(&mut chunk).await.unwrap();
                    data.extend(&chunk[..n]);
                }
                let body: serde_json::Value =
                    serde_json::from_slice(&data[split..split + size]).unwrap();
                let tokenize = data.starts_with(b"POST /tokenize ");
                let (content, content_type, finished) = if tokenize {
                    let count = body["content"].as_str().unwrap().chars().count() / 3;
                    (
                        serde_json::json!({"tokens":vec![0;count]}).to_string(),
                        "application/json",
                        false,
                    )
                } else {
                    let structured = body["stream"] == false;
                    recorded.lock().unwrap().push(body);
                    let (text, reason) = replies.pop_front().unwrap();
                    if structured {
                        (serde_json::json!({"choices":[{"message":{"content":text},"finish_reason":reason}]}).to_string(),"application/json",replies.is_empty())
                    } else {
                        (
                            format!(
                                "data: {}\n\ndata: [DONE]\n\n",
                                serde_json::json!({"choices":[{"delta":{"content":text},"finish_reason":reason}]})
                            ),
                            "text/event-stream",
                            replies.is_empty(),
                        )
                    }
                };
                let response=format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{content}",content.len());
                let _ = socket.write_all(response.as_bytes()).await;
                if finished {
                    break;
                }
            }
        });
        (
            ModelClient {
                client: Client::builder().no_proxy().build().unwrap(),
                endpoint,
                token: "local-test".into(),
            },
            requests,
            task,
        )
    }
    fn messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: "system".into(),
                content: "Answer completely".into(),
                images: vec![],
            },
            ChatMessage {
                role: "user".into(),
                content: "Current question".into(),
                images: vec![],
            },
        ]
    }
    #[tokio::test]
    async fn memory_json_uses_the_pinned_runtime_schema_wrapper() {
        let (client, requests, task) = server(vec![("{\"memories\":[]}", Some("stop"))]).await;
        let schema =
            serde_json::json!({"type":"object","properties":{"memories":{"type":"array"}}});
        let out = client
            .generate_json(messages(), schema.clone(), CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(out, "{\"memories\":[]}");
        {
            let requests = requests.lock().unwrap();
            assert_eq!(
                requests[0]["response_format"]["json_schema"]["schema"],
                schema
            );
            assert_eq!(
                requests[0]["chat_template_kwargs"]["enable_thinking"],
                false
            );
        }
        task.await.unwrap();
    }
    #[tokio::test]
    async fn truncated_memory_json_is_not_a_completed_extraction() {
        let (client, _, task) = server(vec![("{\"memories\":[", Some("length"))]).await;
        assert!(client
            .generate_json(messages(), serde_json::json!({}), CancellationToken::new())
            .await
            .is_err());
        task.await.unwrap();
    }
    #[tokio::test]
    async fn cancelled_memory_job_does_not_start_an_http_request() {
        let (client, requests, task) = server(vec![("{}", Some("stop"))]).await;
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            client
                .generate_json(messages(), serde_json::json!({}), cancel)
                .await,
            Err(Error::Cancelled)
        ));
        assert!(requests.lock().unwrap().is_empty());
        task.abort();
    }
    #[tokio::test]
    async fn length_finish_continues_but_stop_completes() {
        let (client, requests, task) =
            server(vec![("Birinci ", Some("length")), ("son.", Some("stop"))]).await;
        let mut streamed = String::new();
        let mut progress = Vec::new();
        let output = client
            .generate_with_progress(
                messages(),
                CancellationToken::new(),
                &AppSettings::default(),
                |t| streamed.push_str(&t),
                |s| progress.push(s),
            )
            .await
            .unwrap();
        assert_eq!(output, "Birinci son.");
        assert_eq!(streamed, output);
        assert_eq!(progress.len(), 1);
        assert_eq!(requests.lock().unwrap().len(), 2);
        assert_eq!(
            requests.lock().unwrap()[1]["messages"][2]["content"],
            "Birinci "
        );
        task.await.unwrap();
    }
    #[tokio::test]
    async fn done_without_finish_reason_is_not_success() {
        let (client, _, task) = server(vec![("Yarım", None)]).await;
        assert!(client
            .generate_with_settings(
                messages(),
                CancellationToken::new(),
                &AppSettings::default(),
                |_| {}
            )
            .await
            .is_err());
        task.await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_prevents_another_segment() {
        let (client, requests, task) = server(vec![("Yarım", Some("length"))]).await;
        let cancel = CancellationToken::new();
        let copied = cancel.clone();
        assert!(matches!(
            client
                .generate_with_settings(messages(), cancel, &AppSettings::default(), move |_| {
                    copied.cancel()
                })
                .await,
            Err(Error::Cancelled)
        ));
        assert_eq!(requests.lock().unwrap().len(), 1);
        task.await.unwrap();
    }
    #[tokio::test]
    async fn endless_length_is_visible_as_limit() {
        let (client, _, task) = server(vec![("parça", Some("length")); 16]).await;
        assert!(matches!(
            client
                .generate_with_settings(
                    messages(),
                    CancellationToken::new(),
                    &AppSettings::default(),
                    |_| {}
                )
                .await,
            Err(Error::OutputLimit)
        ));
        task.await.unwrap();
    }
    #[tokio::test]
    async fn trims_old_context_without_losing_current_question() {
        let (client, requests, task) = server(vec![("Done", Some("stop"))]).await;
        let mut input = messages();
        input.insert(
            1,
            ChatMessage {
                role: "user".into(),
                content: "old".repeat(20000),
                images: vec![],
            },
        );
        input.insert(
            2,
            ChatMessage {
                role: "assistant".into(),
                content: "old answer".into(),
                images: vec![],
            },
        );
        client
            .generate_with_settings(
                input,
                CancellationToken::new(),
                &AppSettings::default(),
                |_| {},
            )
            .await
            .unwrap();
        assert_eq!(
            requests.lock().unwrap()[0]["messages"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            requests.lock().unwrap()[0]["messages"][1]["content"],
            "Current question"
        );
        task.await.unwrap();
    }
}

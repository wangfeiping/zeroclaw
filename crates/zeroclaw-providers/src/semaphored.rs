//! Provider wrapper that caps concurrent in-flight requests.
//!
//! Local/self-hosted model servers (Ollama, llama.cpp, vLLM on modest
//! hardware) often can't serve multiple inference requests in parallel
//! without exhausting GPU/CPU/VRAM. `SemaphoredProvider` wraps any
//! `Provider` and serializes (or caps) calls through a `tokio::sync::Semaphore`,
//! so the rest of the system (channel dispatch, per-session queues) can keep
//! dispatching concurrently while requests against this specific provider
//! queue up behind the configured limit.

use super::Provider;
use super::traits::{
    ChatMessage, ChatRequest, ChatResponse, ProviderCapabilities, StreamChunk, StreamEvent,
    StreamOptions, StreamResult, ToolsPayload,
};
use async_trait::async_trait;
use futures_util::{StreamExt, stream};
use std::sync::Arc;
use zeroclaw_api::tool::ToolSpec;

pub struct SemaphoredProvider {
    inner: Box<dyn Provider>,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl SemaphoredProvider {
    /// Wrap `inner` so no more than `max_concurrent` requests run at once.
    /// `max_concurrent == 0` is treated as `1` (a fully disabled provider
    /// would be a config footgun, not a useful throttle).
    pub fn new(inner: Box<dyn Provider>, max_concurrent: usize) -> Self {
        Self {
            inner,
            semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent.max(1))),
        }
    }

    /// Gate a `'static` stream behind a semaphore permit, held for the
    /// stream's lifetime. The permit is acquired inside the spawned task
    /// rather than before returning, so callers that never poll the
    /// returned stream don't hold a permit forever.
    fn gate_stream<T: Send + 'static>(
        &self,
        mut inner_stream: stream::BoxStream<'static, T>,
    ) -> stream::BoxStream<'static, T> {
        let semaphore = Arc::clone(&self.semaphore);
        let (tx, rx) = tokio::sync::mpsc::channel::<T>(100);

        tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await;
            while let Some(item) = inner_stream.next().await {
                if tx.send(item).await.is_err() {
                    break;
                }
            }
        });

        stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed()
    }
}

#[async_trait]
impl Provider for SemaphoredProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        self.inner.capabilities()
    }

    fn default_temperature(&self) -> f64 {
        self.inner.default_temperature()
    }

    fn default_max_tokens(&self) -> u32 {
        self.inner.default_max_tokens()
    }

    fn default_timeout_secs(&self) -> u64 {
        self.inner.default_timeout_secs()
    }

    fn default_base_url(&self) -> Option<&str> {
        self.inner.default_base_url()
    }

    fn default_wire_api(&self) -> &str {
        self.inner.default_wire_api()
    }

    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        self.inner.convert_tools(tools)
    }

    fn supports_native_tools(&self) -> bool {
        self.inner.supports_native_tools()
    }

    fn supports_vision(&self) -> bool {
        self.inner.supports_vision()
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    fn supports_streaming_tool_events(&self) -> bool {
        self.inner.supports_streaming_tool_events()
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        let _permit = self.semaphore.acquire().await;
        self.inner.warmup().await
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: Option<f64>,
    ) -> anyhow::Result<String> {
        let _permit = self.semaphore.acquire().await;
        self.inner
            .chat_with_system(system_prompt, message, model, temperature)
            .await
    }

    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        let _permit = self.semaphore.acquire().await;
        self.inner.list_models().await
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: Option<f64>,
    ) -> anyhow::Result<String> {
        let _permit = self.semaphore.acquire().await;
        self.inner
            .chat_with_history(messages, model, temperature)
            .await
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: Option<f64>,
    ) -> anyhow::Result<ChatResponse> {
        let _permit = self.semaphore.acquire().await;
        self.inner.chat(request, model, temperature).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        model: &str,
        temperature: Option<f64>,
    ) -> anyhow::Result<ChatResponse> {
        let _permit = self.semaphore.acquire().await;
        self.inner
            .chat_with_tools(messages, tools, model, temperature)
            .await
    }

    fn stream_chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: Option<f64>,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let inner_stream =
            self.inner
                .stream_chat_with_system(system_prompt, message, model, temperature, options);
        self.gate_stream(inner_stream)
    }

    fn stream_chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: Option<f64>,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
        let inner_stream =
            self.inner
                .stream_chat_with_history(messages, model, temperature, options);
        self.gate_stream(inner_stream)
    }

    fn stream_chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: Option<f64>,
        options: StreamOptions,
    ) -> stream::BoxStream<'static, StreamResult<StreamEvent>> {
        let inner_stream = self.inner.stream_chat(request, model, temperature, options);
        self.gate_stream(inner_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct SlowProvider {
        in_flight: Arc<AtomicUsize>,
        max_observed: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Provider for SlowProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: Option<f64>,
        ) -> anyhow::Result<String> {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_observed.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok("ok".to_string())
        }
    }

    #[tokio::test]
    async fn caps_concurrent_chat_calls() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));
        let inner = SlowProvider {
            in_flight: Arc::clone(&in_flight),
            max_observed: Arc::clone(&max_observed),
        };
        let provider = Arc::new(SemaphoredProvider::new(Box::new(inner), 1));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let provider = Arc::clone(&provider);
            handles.push(tokio::spawn(async move {
                provider
                    .chat_with_system(None, "hi", "model", None)
                    .await
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(max_observed.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn zero_max_concurrent_is_treated_as_one() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));
        let inner = SlowProvider {
            in_flight: Arc::clone(&in_flight),
            max_observed: Arc::clone(&max_observed),
        };
        let provider = SemaphoredProvider::new(Box::new(inner), 0);
        assert_eq!(provider.semaphore.available_permits(), 1);
        provider
            .chat_with_system(None, "hi", "model", None)
            .await
            .unwrap();
    }
}

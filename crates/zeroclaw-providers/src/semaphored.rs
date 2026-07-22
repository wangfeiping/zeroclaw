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
    /// Human-readable provider identity (name, plus base URL when a custom
    /// one is configured) used to tag the WARN-level call log below. Callers
    /// opted into `max_concurrent` because their provider is resource-
    /// constrained (typically a local model server), so every call is logged
    /// loud enough to show up without cranking global log verbosity.
    provider_label: String,
}

impl SemaphoredProvider {
    /// Wrap `inner` so no more than `max_concurrent` requests run at once.
    /// `max_concurrent == 0` is treated as `1` (a fully disabled provider
    /// would be a config footgun, not a useful throttle).
    pub fn new(inner: Box<dyn Provider>, max_concurrent: usize, provider_label: String) -> Self {
        Self {
            inner,
            semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent.max(1))),
            provider_label,
        }
    }

    /// Log a single outbound AI API call at WARN. Only reached when
    /// `max_concurrent` is configured for this provider, so the elevated
    /// level is intentional — operators who opted into throttling a
    /// resource-constrained local model server want these calls visible
    /// without turning up global log verbosity.
    #[allow(clippy::too_many_arguments)]
    fn log_call(
        &self,
        api: &str,
        model: &str,
        temperature: Option<f64>,
        message_count: usize,
        tool_count: usize,
        streaming: bool,
    ) {
        tracing::warn!(
            provider = %self.provider_label,
            api,
            model,
            temperature = ?temperature,
            message_count,
            tool_count,
            streaming,
            "AI API call (provider max_concurrent limit is active)"
        );
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
        self.log_call("chat_with_system", model, temperature, 1, 0, false);
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
        self.log_call(
            "chat_with_history",
            model,
            temperature,
            messages.len(),
            0,
            false,
        );
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
        self.log_call(
            "chat",
            model,
            temperature,
            request.messages.len(),
            request.tools.map_or(0, <[_]>::len),
            false,
        );
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
        self.log_call(
            "chat_with_tools",
            model,
            temperature,
            messages.len(),
            tools.len(),
            false,
        );
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
        self.log_call("stream_chat_with_system", model, temperature, 1, 0, true);
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
        self.log_call(
            "stream_chat_with_history",
            model,
            temperature,
            messages.len(),
            0,
            true,
        );
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
        self.log_call(
            "stream_chat",
            model,
            temperature,
            request.messages.len(),
            request.tools.map_or(0, <[_]>::len),
            true,
        );
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
        let provider = Arc::new(SemaphoredProvider::new(
            Box::new(inner),
            1,
            "test-provider".to_string(),
        ));

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
        let provider = SemaphoredProvider::new(Box::new(inner), 0, "test-provider".to_string());
        assert_eq!(provider.semaphore.available_permits(), 1);
        provider
            .chat_with_system(None, "hi", "model", None)
            .await
            .unwrap();
    }

    /// Minimal hand-rolled `Subscriber` that just records whether a WARN
    /// event fired and its formatted message — enough to prove the call
    /// log exists without pulling in `tracing-subscriber` as a dev-dep.
    struct WarnCapture {
        fired: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl tracing::Subscriber for WarnCapture {
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            metadata.level() <= &tracing::Level::WARN
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                struct MessageVisitor(String);
                impl tracing::field::Visit for MessageVisitor {
                    fn record_debug(
                        &mut self,
                        field: &tracing::field::Field,
                        value: &dyn std::fmt::Debug,
                    ) {
                        if field.name() == "message" {
                            self.0 = format!("{value:?}");
                        }
                    }
                }
                let mut visitor = MessageVisitor(String::new());
                event.record(&mut visitor);
                self.fired.lock().unwrap().push(visitor.0);
            }
        }

        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    #[tokio::test]
    async fn max_concurrent_call_is_logged_at_warn() {
        let inner = SlowProvider {
            in_flight: Arc::new(AtomicUsize::new(0)),
            max_observed: Arc::new(AtomicUsize::new(0)),
        };
        let provider = SemaphoredProvider::new(Box::new(inner), 1, "ollama".to_string());

        let fired = Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = WarnCapture {
            fired: Arc::clone(&fired),
        };
        let _guard = tracing::subscriber::set_default(subscriber);

        provider
            .chat_with_system(Some("sys"), "hi", "llama3", Some(0.5))
            .await
            .unwrap();

        let messages = fired.lock().unwrap();
        assert_eq!(messages.len(), 1, "expected exactly one WARN event");
        assert!(messages[0].contains("AI API call"));
    }
}

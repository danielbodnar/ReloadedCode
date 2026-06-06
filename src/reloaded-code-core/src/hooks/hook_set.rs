//! HookSet — container and dispatch for all registered hooks and lifecycle events.

use crate::hooks::{
    EndReason, SessionCompactFn, SessionContext, SessionEndFn, SessionStartFn, ToolCallContext,
    ToolExecutor, ToolHook, ToolHookFuture, ToolOriginal, ToolRequest, INLINE_CAP,
};
use std::fmt;
use std::sync::Arc;
use tinyvec::ArrayVec;

/// All registered hooks and lifecycle events, stored per point.
#[derive(Clone, Default)]
pub struct HookSet {
    pub(super) tool_hooks: Vec<Arc<dyn ToolHook>>,
    pub(super) session_start: ArrayVec<[Option<SessionStartFn>; INLINE_CAP]>,
    pub(super) session_end: ArrayVec<[Option<SessionEndFn>; INLINE_CAP]>,
    pub(super) session_compact: ArrayVec<[Option<SessionCompactFn>; INLINE_CAP]>,
}

impl HookSet {
    /// Returns `true` if no hooks are registered at any point.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tool_hooks.is_empty()
            && self.session_start.is_empty()
            && self.session_end.is_empty()
            && self.session_compact.is_empty()
    }

    /// Returns `true` if no tool hooks are registered.
    #[inline]
    #[must_use]
    pub fn tool_hooks_is_empty(&self) -> bool {
        self.tool_hooks.is_empty()
    }

    /// Returns registered tool hooks in dispatch order.
    #[inline]
    #[must_use]
    pub fn tool_hooks(&self) -> &[Arc<dyn ToolHook>] {
        &self.tool_hooks
    }

    /// Returns a new builder for constructing a `HookSet`.
    #[inline]
    #[must_use]
    pub fn builder() -> crate::hooks::builder::HookSetBuilder {
        crate::hooks::builder::HookSetBuilder::new()
    }

    /// Dispatches a tool call through the hook chain.
    ///
    /// If no tool hooks are registered, this calls the real tool directly.
    #[inline]
    pub fn dispatch_tool<'a>(
        &'a self,
        ctx: &'a ToolCallContext<'a>,
        req: ToolRequest,
        real_tool: &'a dyn ToolExecutor,
    ) -> ToolHookFuture<'a> {
        if self.tool_hooks.is_empty() {
            return real_tool.execute(ctx, req);
        }

        ToolOriginal::new(&self.tool_hooks, real_tool).call(ctx, req)
    }

    /// Dispatches session-start events.
    #[inline]
    pub fn dispatch_session_start(&self, ctx: &SessionContext<'_>) {
        for event in self.session_start.iter().flatten() {
            event(ctx);
        }
    }

    /// Dispatches session-end events.
    #[inline]
    pub fn dispatch_session_end(&self, ctx: &SessionContext<'_>, reason: EndReason) {
        for event in self.session_end.iter().flatten() {
            event(ctx, reason);
        }
    }

    /// Dispatches session-compact events.
    #[inline]
    pub fn dispatch_session_compact(&self, ctx: &SessionContext<'_>) {
        for event in self.session_compact.iter().flatten() {
            event(ctx);
        }
    }
}

impl fmt::Debug for HookSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HookSet")
            .field("tool_hooks", &self.tool_hooks.len())
            .field("session_start", &self.session_start.len())
            .field("session_end", &self.session_end.len())
            .field("session_compact", &self.session_compact.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolOutput;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn ready(output: impl Into<ToolOutput>) -> ToolHookFuture<'static> {
        let output = output.into();
        Box::pin(async move { Ok(output) })
    }

    #[test]
    fn hook_set_default_is_empty() {
        let hooks = HookSet::default();
        assert!(hooks.is_empty());
        assert!(hooks.tool_hooks_is_empty());
    }

    #[tokio::test]
    async fn dispatch_tool_empty_calls_real_tool_directly() {
        struct RealTool;

        impl ToolExecutor for RealTool {
            fn execute<'a>(
                &'a self,
                _ctx: &'a ToolCallContext<'a>,
                req: ToolRequest,
            ) -> ToolHookFuture<'a> {
                let content = req.args["value"].as_str().unwrap().to_string();
                Box::pin(async move { Ok(ToolOutput::new(content)) })
            }
        }

        let hooks = HookSet::default();
        let ctx = ToolCallContext {
            tool_name: "echo",
            agent_name: "coder",
            run_id: "r1",
        };
        let output = hooks
            .dispatch_tool(&ctx, ToolRequest::new(json!({"value": "ok"})), &RealTool)
            .await
            .unwrap();

        assert_eq!(output.content, "ok");
    }

    #[tokio::test]
    async fn dispatch_tool_hooks_wrap_real_tool() {
        struct Prefix;
        struct Suffix;
        struct RealTool;

        impl ToolHook for Prefix {
            fn hook<'a>(
                &'a self,
                ctx: &'a ToolCallContext<'a>,
                mut req: ToolRequest,
                original: ToolOriginal<'a>,
            ) -> ToolHookFuture<'a> {
                Box::pin(async move {
                    req.args["value"] =
                        json!(format!("pre-{}", req.args["value"].as_str().unwrap()));
                    let mut output = original.call(ctx, req).await?;
                    output.content.push_str("-post");
                    Ok(output)
                })
            }
        }

        impl ToolHook for Suffix {
            fn hook<'a>(
                &'a self,
                ctx: &'a ToolCallContext<'a>,
                mut req: ToolRequest,
                original: ToolOriginal<'a>,
            ) -> ToolHookFuture<'a> {
                Box::pin(async move {
                    req.args["value"] =
                        json!(format!("{}-inner", req.args["value"].as_str().unwrap()));
                    let mut output = original.call(ctx, req).await?;
                    output.content.push_str("-innerpost");
                    Ok(output)
                })
            }
        }

        impl ToolExecutor for RealTool {
            fn execute<'a>(
                &'a self,
                _ctx: &'a ToolCallContext<'a>,
                req: ToolRequest,
            ) -> ToolHookFuture<'a> {
                let content = req.args["value"].as_str().unwrap().to_string();
                Box::pin(async move { Ok(ToolOutput::new(content)) })
            }
        }

        let hooks = crate::hooks::builder::HookSetBuilder::new()
            .tool_hook(Prefix)
            .tool_hook(Suffix)
            .build();
        let ctx = ToolCallContext {
            tool_name: "echo",
            agent_name: "coder",
            run_id: "r1",
        };
        let output = hooks
            .dispatch_tool(&ctx, ToolRequest::new(json!({"value": "x"})), &RealTool)
            .await
            .unwrap();

        assert_eq!(output.content, "pre-x-inner-innerpost-post");
    }

    #[tokio::test]
    async fn dispatch_tool_hook_can_block_without_calling_original() {
        struct Block;
        struct RealTool;

        impl ToolHook for Block {
            fn hook<'a>(
                &'a self,
                _ctx: &'a ToolCallContext<'a>,
                _req: ToolRequest,
                _original: ToolOriginal<'a>,
            ) -> ToolHookFuture<'a> {
                Box::pin(async { Ok(ToolOutput::new("blocked")) })
            }
        }

        impl ToolExecutor for RealTool {
            fn execute<'a>(
                &'a self,
                _ctx: &'a ToolCallContext<'a>,
                _req: ToolRequest,
            ) -> ToolHookFuture<'a> {
                ready("should not run")
            }
        }

        let hooks = crate::hooks::builder::HookSetBuilder::new()
            .tool_hook(Block)
            .build();
        let ctx = ToolCallContext {
            tool_name: "bash",
            agent_name: "coder",
            run_id: "r1",
        };
        let output = hooks
            .dispatch_tool(&ctx, ToolRequest::new(json!({})), &RealTool)
            .await
            .unwrap();

        assert_eq!(output.content, "blocked");
    }

    #[test]
    fn session_events_dispatch() {
        static STARTS: AtomicUsize = AtomicUsize::new(0);
        static ENDS: AtomicUsize = AtomicUsize::new(0);
        static COMPACTS: AtomicUsize = AtomicUsize::new(0);

        fn on_start(_ctx: &SessionContext<'_>) {
            STARTS.fetch_add(1, Ordering::SeqCst);
        }

        fn on_end(_ctx: &SessionContext<'_>, reason: EndReason) {
            assert_eq!(reason, EndReason::Completed);
            ENDS.fetch_add(1, Ordering::SeqCst);
        }

        fn on_compact(_ctx: &SessionContext<'_>) {
            COMPACTS.fetch_add(1, Ordering::SeqCst);
        }

        STARTS.store(0, Ordering::SeqCst);
        ENDS.store(0, Ordering::SeqCst);
        COMPACTS.store(0, Ordering::SeqCst);

        let hooks = crate::hooks::builder::HookSetBuilder::new()
            .on_session_start(on_start)
            .on_session_end(on_end)
            .on_session_compact(on_compact)
            .build();
        let ctx = SessionContext {
            agent_name: "coder",
            run_id: "r1",
        };

        hooks.dispatch_session_start(&ctx);
        hooks.dispatch_session_end(&ctx, EndReason::Completed);
        hooks.dispatch_session_compact(&ctx);

        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        assert_eq!(ENDS.load(Ordering::SeqCst), 1);
        assert_eq!(COMPACTS.load(Ordering::SeqCst), 1);
    }
}

//! HookSetBuilder — builder for constructing a [`HookSet`].

use crate::hooks::{HookSet, SessionCompactFn, SessionEndFn, SessionStartFn, ToolHook, INLINE_CAP};
use std::fmt;
use std::sync::Arc;
use tinyvec::ArrayVec;

/// Builder for constructing [`HookSet`].
#[derive(Default)]
pub struct HookSetBuilder {
    pub(super) tool_hooks: Vec<Arc<dyn ToolHook>>,
    pub(super) session_start: ArrayVec<[Option<SessionStartFn>; INLINE_CAP]>,
    pub(super) session_end: ArrayVec<[Option<SessionEndFn>; INLINE_CAP]>,
    pub(super) session_compact: ArrayVec<[Option<SessionCompactFn>; INLINE_CAP]>,
}

impl HookSetBuilder {
    /// Creates a new, empty builder.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a game-style tool hook.
    ///
    /// Hooks run in registration order. Each hook's `original` handle calls
    /// the next registered hook, or the real tool at the end of the chain.
    #[inline]
    #[must_use]
    pub fn tool_hook(mut self, hook: impl ToolHook) -> Self {
        self.tool_hooks.push(Arc::new(hook));
        self
    }

    /// Registers an already shared game-style tool hook.
    #[inline]
    #[must_use]
    pub fn shared_tool_hook(mut self, hook: Arc<dyn ToolHook>) -> Self {
        self.tool_hooks.push(hook);
        self
    }

    /// Registers a session-start event.
    #[inline]
    #[must_use]
    pub fn on_session_start(mut self, event: SessionStartFn) -> Self {
        self.session_start.push(Some(event));
        self
    }

    /// Registers a session-end event.
    #[inline]
    #[must_use]
    pub fn on_session_end(mut self, event: SessionEndFn) -> Self {
        self.session_end.push(Some(event));
        self
    }

    /// Registers a session-compact event.
    #[inline]
    #[must_use]
    pub fn on_session_compact(mut self, event: SessionCompactFn) -> Self {
        self.session_compact.push(Some(event));
        self
    }

    /// Builds the `HookSet` from the configured hooks.
    #[inline]
    #[must_use]
    pub fn build(self) -> HookSet {
        HookSet {
            tool_hooks: self.tool_hooks,
            session_start: self.session_start,
            session_end: self.session_end,
            session_compact: self.session_compact,
        }
    }
}

impl fmt::Debug for HookSetBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HookSetBuilder")
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
    use crate::hooks::tool_hook::{ToolCallContext, ToolHookFuture, ToolOriginal, ToolRequest};

    #[test]
    fn hook_set_builder_new_produces_empty() {
        let hooks = HookSetBuilder::new().build();
        assert!(hooks.is_empty());
    }

    #[test]
    fn hook_set_builder_roundtrip() {
        let hooks = HookSet::builder().build();
        assert!(hooks.is_empty());
    }

    #[test]
    fn tool_hook_registration_makes_hook_set_non_empty() {
        struct Noop;

        impl ToolHook for Noop {
            fn hook<'a>(
                &'a self,
                ctx: &'a ToolCallContext<'a>,
                req: ToolRequest,
                original: ToolOriginal<'a>,
            ) -> ToolHookFuture<'a> {
                original.call(ctx, req)
            }
        }

        let hooks = HookSetBuilder::new().tool_hook(Noop).build();
        assert!(!hooks.is_empty());
        assert!(!hooks.tool_hooks_is_empty());
        assert_eq!(hooks.tool_hooks().len(), 1);
    }
}

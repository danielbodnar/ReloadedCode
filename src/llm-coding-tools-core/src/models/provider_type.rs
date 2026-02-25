/// Provider behavior profile used by model resolver logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ProviderType {
    /// Unknown or unsupported provider package.
    #[default]
    Unknown,
    /// OpenAI chat-completions provider.
    OpenAiCompletions,
    /// OpenAI Responses API provider.
    OpenAiResponses,
    /// Anthropic provider.
    Anthropic,
    /// Google/Gemini provider.
    Google,
    /// Groq provider.
    Groq,
    /// Mistral provider.
    Mistral,
    /// Ollama provider.
    Ollama,
    /// AWS Bedrock provider.
    Bedrock,
    /// Azure-style provider where a base URL is required.
    Azure,
    /// OpenRouter provider.
    OpenRouter,
    /// Hugging Face provider.
    HuggingFace,
    /// Cohere provider.
    Cohere,
    /// ChatGPT OAuth provider.
    ChatGptOAuth,
    /// Claude Code OAuth provider.
    ClaudeCodeOAuth,
    /// Antigravity provider.
    Antigravity,
}

impl ProviderType {
    /// Encodes this provider type to its packed `u8` representation.
    #[inline]
    pub(crate) const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Decodes a packed `u8` provider type.
    #[inline]
    pub(crate) const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::OpenAiCompletions,
            2 => Self::OpenAiResponses,
            3 => Self::Anthropic,
            4 => Self::Google,
            5 => Self::Groq,
            6 => Self::Mistral,
            7 => Self::Ollama,
            8 => Self::Bedrock,
            9 => Self::Azure,
            10 => Self::OpenRouter,
            11 => Self::HuggingFace,
            12 => Self::Cohere,
            13 => Self::ChatGptOAuth,
            14 => Self::ClaudeCodeOAuth,
            15 => Self::Antigravity,
            _ => Self::Unknown,
        }
    }

    /// Returns true when this provider requires an API key.
    #[inline]
    pub const fn requires_api_key(self) -> bool {
        !matches!(self, Self::Ollama)
    }

    /// Returns true when this provider supports an explicit base URL override.
    #[inline]
    pub const fn supports_base_url(self) -> bool {
        !matches!(self, Self::Bedrock)
    }

    /// Returns true when this provider requires a base URL.
    #[inline]
    pub const fn requires_base_url(self) -> bool {
        matches!(self, Self::Azure)
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderType;

    #[test]
    fn unknown_is_default_variant() {
        assert_eq!(ProviderType::default(), ProviderType::Unknown);
    }

    #[test]
    fn azure_requires_base_url() {
        assert!(ProviderType::Azure.requires_base_url());
    }

    #[test]
    fn ollama_does_not_require_api_key() {
        assert!(!ProviderType::Ollama.requires_api_key());
    }

    #[test]
    fn packed_roundtrip_uses_u8_encoding() {
        assert_eq!(
            ProviderType::from_u8(ProviderType::Azure.to_u8()),
            ProviderType::Azure
        );
    }

    #[test]
    fn unknown_is_used_for_invalid_discriminant() {
        assert_eq!(ProviderType::from_u8(u8::MAX), ProviderType::Unknown);
    }
}

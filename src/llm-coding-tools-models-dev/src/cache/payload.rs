use crate::error::{CatalogError, CatalogResult};
use llm_coding_tools_core::models::{
    Modality, ModelCatalog, ModelInfo, ProviderIdx, ProviderInfo, ProviderModelSource,
    ProviderSource, ProviderType,
};

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub(crate) struct CatalogCachePayload {
    pub(crate) providers: Vec<CachedProviderRow>,
    pub(crate) models: Vec<CachedModelRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub(crate) struct CachedProviderRow {
    pub(crate) provider_key: String,
    pub(crate) api_url: String,
    pub(crate) env_vars: Vec<String>,
    pub(crate) api_type: ProviderType,
}

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub(crate) struct CachedModelRow {
    pub(crate) provider_idx: ProviderIdx,
    pub(crate) model_key: String,
    pub(crate) modalities_bits: u8,
    pub(crate) max_input: u32,
    pub(crate) max_output: u32,
    pub(crate) temperature: Option<f32>,
    pub(crate) top_p: Option<f32>,
}

pub(crate) fn encode_cache_payload(payload: &CatalogCachePayload) -> Vec<u8> {
    bitcode::encode(payload)
}

pub(crate) fn decode_cache_payload(bytes: &[u8]) -> CatalogResult<CatalogCachePayload> {
    bitcode::decode(bytes).map_err(|error| CatalogError::BitcodeDecode(error.to_string()))
}

pub(crate) fn catalog_from_cache_payload(
    payload: CatalogCachePayload,
) -> CatalogResult<ModelCatalog> {
    let CatalogCachePayload { providers, models } = payload;

    let mut provider_sources = Vec::with_capacity(providers.len());
    for row in providers {
        provider_sources.push(ProviderSource {
            provider_key: row.provider_key,
            provider: ProviderInfo {
                api_url: row.api_url,
                env_vars: row.env_vars,
                api_type: row.api_type,
            },
        });
    }

    let mut model_sources = Vec::with_capacity(models.len());
    for row in &models {
        let provider_source =
            provider_sources
                .get(row.provider_idx.as_usize())
                .ok_or(CatalogError::CacheFormat(
                    "provider index out of range in cache payload",
                ))?;

        model_sources.push(ProviderModelSource {
            provider_key: provider_source.provider_key.as_str(),
            model_key: row.model_key.as_str(),
            model: ModelInfo {
                modalities: Modality::from_bits_retain(row.modalities_bits),
                max_input: row.max_input,
                max_output: row.max_output,
                temperature: row.temperature,
                top_p: row.top_p,
            },
        });
    }

    Ok(ModelCatalog::build(&provider_sources, &model_sources)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> CatalogCachePayload {
        CatalogCachePayload {
            providers: vec![CachedProviderRow {
                provider_key: "openai".to_string(),
                api_url: "https://api.openai.com/v1".to_string(),
                env_vars: vec!["OPENAI_API_KEY".to_string()],
                api_type: ProviderType::OpenAiCompletions,
            }],
            models: vec![CachedModelRow {
                provider_idx: ProviderIdx::new(0),
                model_key: "gpt-4".to_string(),
                modalities_bits: Modality::TEXT.bits(),
                max_input: 8192,
                max_output: 4096,
                temperature: Some(0.7),
                top_p: Some(0.9),
            }],
        }
    }

    #[test]
    fn payload_round_trip() {
        let original = sample_payload();
        let encoded = encode_cache_payload(&original);
        let decoded = decode_cache_payload(&encoded).expect("decode should succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn catalog_from_payload_reconstructs_provider() {
        let payload = sample_payload();
        let catalog = catalog_from_cache_payload(payload).expect("catalog build should succeed");

        let provider = catalog
            .lookup_provider("openai")
            .expect("provider should exist");
        assert_eq!(provider.api_url, "https://api.openai.com/v1");
        assert_eq!(provider.api_type, ProviderType::OpenAiCompletions);
    }

    #[test]
    fn catalog_from_payload_reconstructs_model() {
        let payload = sample_payload();
        let catalog = catalog_from_cache_payload(payload).expect("catalog build should succeed");

        let model = catalog
            .lookup_provider_model("openai", "gpt-4")
            .expect("model should exist");
        assert_eq!(model.max_input, 8192);
        assert_eq!(model.max_output, 4096);
        assert_eq!(model.modalities, Modality::TEXT);
    }

    #[test]
    fn catalog_from_payload_rejects_out_of_range_provider_idx() {
        let payload = CatalogCachePayload {
            providers: vec![CachedProviderRow {
                provider_key: "test".to_string(),
                api_url: "".to_string(),
                env_vars: vec![],
                api_type: ProviderType::Unknown,
            }],
            models: vec![CachedModelRow {
                provider_idx: ProviderIdx::new(999),
                model_key: "model".to_string(),
                modalities_bits: Modality::TEXT.bits(),
                max_input: 0,
                max_output: 0,
                temperature: None,
                top_p: None,
            }],
        };

        let result = catalog_from_cache_payload(payload);
        assert!(matches!(result, Err(CatalogError::CacheFormat(_))));
    }

    #[test]
    fn all_known_provider_types_round_trip() {
        let types = [
            ProviderType::Unknown,
            ProviderType::OpenAiCompletions,
            ProviderType::OpenAiResponses,
            ProviderType::Anthropic,
            ProviderType::Google,
            ProviderType::Groq,
            ProviderType::Mistral,
            ProviderType::Ollama,
            ProviderType::Bedrock,
            ProviderType::Azure,
            ProviderType::OpenRouter,
            ProviderType::HuggingFace,
            ProviderType::Cohere,
            ProviderType::ChatGptOAuth,
            ProviderType::ClaudeCodeOAuth,
            ProviderType::Antigravity,
        ];

        for provider_type in types {
            let payload = CatalogCachePayload {
                providers: vec![CachedProviderRow {
                    provider_key: "test".to_string(),
                    api_url: "".to_string(),
                    env_vars: vec![],
                    api_type: provider_type,
                }],
                models: vec![],
            };

            let catalog = catalog_from_cache_payload(payload).expect("should succeed");
            let provider = catalog
                .lookup_provider("test")
                .expect("provider should exist");
            assert_eq!(
                provider.api_type, provider_type,
                "provider type should round-trip correctly"
            );
        }
    }
}

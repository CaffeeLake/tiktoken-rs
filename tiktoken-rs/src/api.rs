use anyhow::{anyhow, Result};

use crate::{
    cl100k_base,
    model::get_context_size,
    p50k_base, p50k_edit, r50k_base,
    tokenizer::{get_tokenizer, Tokenizer},
    CoreBPE,
};

/// Calculates the maximum number of tokens available for completion based on the model and prompt provided.
///
/// This function determines the number of tokens left for a completion task, given the model and a prompt string.
/// It first retrieves the context size for the given model and the `CoreBPE` instance for tokenization.
/// Then, it calculates the number of tokens in the prompt using the appropriate tokenizer.
///
/// # Arguments
///
/// * `model` - A string slice representing the model name, e.g., "gpt-3.5-turbo".
/// * `prompt` - A string slice containing the prompt text.
///
/// # Errors
///
/// This function returns an error in the following cases:
///
/// * If there is a failure in creating a `CoreBPE` instance for the specified model.
///
/// # Examples
///
/// ```
/// use tiktoken_rs::get_completion_max_tokens;
///
/// let model = "gpt-3.5-turbo";
/// let prompt = "Translate the following English text to French: '";
/// let max_tokens = get_completion_max_tokens(model, prompt).unwrap();
/// ```
///
/// # Returns
///
/// If successful, the function returns a `Result` containing the maximum number of tokens available for completion,
/// based on the given model and prompt.
pub fn get_completion_max_tokens(model: &str, prompt: &str) -> Result<usize> {
    let context_size = get_context_size(model);
    let bpe = get_bpe_from_model(model)?;
    let prompt_tokens = bpe.encode_with_special_tokens(prompt).len();
    Ok(context_size.saturating_sub(prompt_tokens))
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ChatCompletionRequestMessage {
    /// The role of the author of this message.
    pub role: String,
    /// The contents of the message
    pub content: String,
    /// The name of the user in a multi-user chat
    pub name: Option<String>,
}

/// Calculates the number of tokens for chat completion based on the model and messages provided.
/// Based on https://github.com/openai/openai-cookbook/blob/main/examples/How_to_count_tokens_with_tiktoken.ipynb
pub fn num_tokens_from_messages(
    model: &str,
    messages: &[ChatCompletionRequestMessage],
) -> Result<usize> {
    let tokenizer =
        get_tokenizer(model).ok_or_else(|| anyhow!("No tokenizer found for model {}", model))?;
    if tokenizer != Tokenizer::Cl100kBase {
        anyhow::bail!("Chat completion is only supported chat models")
    }
    let bpe = get_bpe_from_tokenizer(tokenizer)?;

    let (tokens_per_message, tokens_per_name) = if model.starts_with("gpt-3.5") {
        (
            4,  // every message follows <im_start>{role/name}\n{content}<im_end>\n
            -1, // if there's a name, the role is omitted
        )
    } else {
        (3, 1)
    };

    let mut num_tokens: i32 = 0;
    for message in messages {
        num_tokens += tokens_per_message;
        num_tokens += bpe
            .encode_with_special_tokens(&message.role.to_string())
            .len() as i32;
        num_tokens += bpe.encode_with_special_tokens(&message.content).len() as i32;
        if let Some(name) = &message.name {
            num_tokens += bpe.encode_with_special_tokens(name).len() as i32;
            num_tokens += tokens_per_name;
        }
    }
    num_tokens += 3; // every reply is primed with <|start|>assistant<|message|>
    Ok(num_tokens as usize)
}

/// Calculates the maximum number of tokens available for chat completion based on the model and messages provided.
///
/// This function determines the number of tokens left for a chat completion task, given the model and a slice of
/// chat completion request messages. It first retrieves the tokenizer for the given model and checks if chat completion
/// is supported. Then, it calculates the number of tokens in the existing messages using the appropriate tokenizer.
///
/// # Arguments
///
/// * `model` - A string slice representing the model name, e.g., "gpt-3.5-turbo".
/// * `messages` - A slice of `ChatCompletionRequestMessage` instances containing the chat context.
///
/// # Errors
///
/// This function returns an error in the following cases:
///
/// * If there is no tokenizer found for the specified model.
/// * If chat completion is not supported for the specified model.
/// * If there is a failure in creating a `CoreBPE` instance for the specified tokenizer.
///
/// # Example
///
/// ```
/// use tiktoken_rs::{get_chat_completion_max_tokens, ChatCompletionRequestMessage};
///
/// let model = "gpt-3.5-turbo";
/// let messages = vec![
///     ChatCompletionRequestMessage {
///         content: "You are a helpful assistant that only speaks French.".to_string(),
///         role: "system".to_string(),
///         name: None,
///     },
///     ChatCompletionRequestMessage {
///         content: "Hello, how are you?".to_string(),
///         role: "user".to_string(),
///         name: None,
///     },
///     ChatCompletionRequestMessage {
///         content: "Parlez-vous francais?".to_string(),
///         role: "system".to_string(),
///         name: None,
///     },
/// ];
/// let max_tokens = get_chat_completion_max_tokens(model, &messages).unwrap();
/// ```
///
/// # Returns
///
/// If successful, the function returns a `Result` containing the maximum number of tokens available for chat completion,
/// based on the given model and messages.
pub fn get_chat_completion_max_tokens(
    model: &str,
    messages: &[ChatCompletionRequestMessage],
) -> Result<usize> {
    let context_size = get_context_size(model);
    let prompt_tokens = num_tokens_from_messages(model, messages)?;
    Ok(context_size.saturating_sub(prompt_tokens))
}

/// Returns a `CoreBPE` instance corresponding to the tokenizer used by the given model.
///
/// This function first retrieves the tokenizer associated with the specified model name
/// and then maps the tokenizer to the appropriate `CoreBPE` instance, which is used for
/// tokenization in different models.
///
/// # Arguments
///
/// * `model` - A string slice representing the model name for which a `CoreBPE` instance should be retrieved.
///
/// # Errors
///
/// This function returns an error if:
/// * No tokenizer is found for the given model.
/// * There is a failure in creating a `CoreBPE` instance for the tokenizer.
///
/// # Examples
///
/// ```
/// use tiktoken_rs::get_bpe_from_model;
///
/// let model = "gpt-4-0314";
/// let bpe = get_bpe_from_model(model).unwrap();
/// ```
///
/// # Returns
///
/// If successful, the function returns a `Result` containing the `CoreBPE` instance corresponding to the tokenizer used by the given model.
pub fn get_bpe_from_model(model: &str) -> Result<CoreBPE> {
    let tokenizer =
        get_tokenizer(model).ok_or_else(|| anyhow!("No tokenizer found for model {}", model))?;
    let bpe = get_bpe_from_tokenizer(tokenizer)?;
    Ok(bpe)
}

/// Returns a `CoreBPE` instance corresponding to the given tokenizer.
///
/// This function is responsible for mapping a `Tokenizer` enum variant to the appropriate
/// `CoreBPE` instance, which is used for tokenization in different models.
///
/// # Arguments
///
/// * `tokenizer` - A `Tokenizer` enum variant representing the tokenizer for which a `CoreBPE` instance should be retrieved.
///
/// # Errors
///
/// This function returns an error if there is a failure in creating a `CoreBPE` instance for the specified tokenizer.
///
/// # Examples
///
/// ```
/// use tiktoken_rs::get_bpe_from_tokenizer;
/// use tiktoken_rs::tokenizer::Tokenizer;
///
/// let tokenizer = Tokenizer::Cl100kBase;
/// let bpe = get_bpe_from_tokenizer(tokenizer).unwrap();
/// ```
///
/// # Returns
///
/// If successful, the function returns a `Result` containing the `CoreBPE` instance corresponding to the given tokenizer.
pub fn get_bpe_from_tokenizer(tokenizer: Tokenizer) -> Result<CoreBPE> {
    match tokenizer {
        Tokenizer::Cl100kBase => cl100k_base(),
        Tokenizer::R50kBase => r50k_base(),
        Tokenizer::P50kBase => p50k_base(),
        Tokenizer::P50kEdit => p50k_edit(),
        Tokenizer::Gpt2 => r50k_base(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bpe_from_tokenizer() {
        let bpe = get_bpe_from_tokenizer(Tokenizer::Cl100kBase).unwrap();
        assert_eq!(bpe.decode(vec!(15339)).unwrap(), "hello");
    }

    #[test]
    fn test_num_tokens_from_messages() {
        let messages = vec![
            ChatCompletionRequestMessage {
                role: "system".to_string(),
                name: None,
                content: "You are a helpful, pattern-following assistant that translates corporate jargon into plain English.".to_string(),
            },
            ChatCompletionRequestMessage {
                role: "system".to_string(),
                name: Some("example_user".to_string()),
                content: "New synergies will help drive top-line growth.".to_string(),
            },
            ChatCompletionRequestMessage {
                role: "system".to_string(),
                name: Some("example_assistant".to_string()),
                content: "Things working well together will increase revenue.".to_string(),
            },
            ChatCompletionRequestMessage {
                role: "system".to_string(),
                name: Some("example_user".to_string()),
                content: "Let's circle back when we have more bandwidth to touch base on opportunities for increased leverage.".to_string(),
            },
            ChatCompletionRequestMessage {
                role: "system".to_string(),
                name: Some("example_assistant".to_string()),
                content: "Let's talk later when we're less busy about how to do better.".to_string(),
            },
            ChatCompletionRequestMessage {
                role: "user".to_string(),
                name: None,
                content: "This late pivot means we don't have time to boil the ocean for the client deliverable.".to_string(),
            },
        ];
        let num_tokens = num_tokens_from_messages("gpt-3.5-turbo-0301", &messages).unwrap();
        assert_eq!(num_tokens, 127);

        let num_tokens = num_tokens_from_messages("gpt-4-0314", &messages).unwrap();
        assert_eq!(num_tokens, 129);
    }

    #[test]
    fn test_get_chat_completion_max_tokens() {
        let model = "gpt-3.5-turbo";
        let messages = vec![
            ChatCompletionRequestMessage {
                content: "You are a helpful assistant that only speaks French.".to_string(),
                role: "system".to_string(),
                name: None,
            },
            ChatCompletionRequestMessage {
                content: "Hello, how are you?".to_string(),
                role: "user".to_string(),
                name: None,
            },
            ChatCompletionRequestMessage {
                content: "Parlez-vous francais?".to_string(),
                role: "system".to_string(),
                name: None,
            },
        ];
        let max_tokens = get_chat_completion_max_tokens(model, &messages).unwrap();
        assert!(max_tokens > 0);
    }

    #[test]
    fn test_get_completion_max_tokens() {
        let model = "gpt-3.5-turbo";
        let prompt = "Translate the following English text to French: '";
        let max_tokens = get_completion_max_tokens(model, prompt).unwrap();
        assert!(max_tokens > 0);
    }
}

#[cfg(feature = "async-openai")]
pub mod async_openai {
    use anyhow::Result;

    impl From<&async_openai::types::ChatCompletionRequestMessage>
        for super::ChatCompletionRequestMessage
    {
        fn from(m: &async_openai::types::ChatCompletionRequestMessage) -> Self {
            Self {
                role: m.role.to_string(),
                name: m.name.clone(),
                content: m.content.clone(),
            }
        }
    }

    pub fn num_tokens_from_messages(
        model: &str,
        messages: &[async_openai::types::ChatCompletionRequestMessage],
    ) -> Result<usize> {
        let messages = messages.iter().map(|m| m.into()).collect::<Vec<_>>();
        super::num_tokens_from_messages(model, &messages)
    }

    pub fn get_chat_completion_max_tokens(
        model: &str,
        messages: &[async_openai::types::ChatCompletionRequestMessage],
    ) -> Result<usize> {
        let messages = messages.iter().map(|m| m.into()).collect::<Vec<_>>();
        super::get_chat_completion_max_tokens(model, &messages)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_num_tokens_from_messages() {
            let messages = &[async_openai::types::ChatCompletionRequestMessage {
                role: async_openai::types::Role::System,
                name: None,
                content: "You are a helpful, pattern-following assistant that translates corporate jargon into plain English.".to_string(),
            }];
            let num_tokens = num_tokens_from_messages("gpt-3.5-turbo-0301", messages).unwrap();
            assert!(num_tokens > 0);
        }

        #[test]
        fn test_get_chat_completion_max_tokens() {
            let model = "gpt-3.5-turbo";
            let messages = &[async_openai::types::ChatCompletionRequestMessage {
                content: "You are a helpful assistant that only speaks French.".to_string(),
                role: async_openai::types::Role::System,
                name: None,
            }];
            let max_tokens = get_chat_completion_max_tokens(model, messages).unwrap();
            assert!(max_tokens > 0);
        }
    }
}
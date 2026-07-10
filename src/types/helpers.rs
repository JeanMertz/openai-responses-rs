use super::{ContentInput, ContentItem, Input, InputItem, InputListItem, InputMessage, Model};

macro_rules! string_variant {
    ($name:ident, $variant:ident) => {
        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::$variant(value)
            }
        }
        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::from(value.to_string())
            }
        }
    };
}

impl From<InputMessage> for InputListItem {
    fn from(value: InputMessage) -> Self {
        Self::Message(value)
    }
}
impl From<InputItem> for InputListItem {
    fn from(value: InputItem) -> Self {
        Self::Item(value)
    }
}

string_variant!(Input, Text);
string_variant!(ContentInput, Text);

impl From<String> for Model {
    fn from(s: String) -> Self {
        match s.as_str() {
            "o1" => Self::o1,
            "gpt-4" => Self::GPT4,
            "gpt-4o" => Self::GPT4o,
            "o1-mini" => Self::o1Mini,
            "o3-mini" => Self::o3Mini,
            "gpt-4o-mini" => Self::GPT4oMini,
            "gpt-4o-turbo" => Self::GPT4Turbo,
            "gpt-3.5-turbo" => Self::GPT3_5Turbo,
            "gpt-4.5-preview" => Self::GPT4_5Preview,
            "computer-use-preview" => Self::ComputerUsePreview,
            _ => Self::Other(s),
        }
    }
}

impl From<&str> for Model {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

impl From<String> for ContentItem {
    fn from(value: String) -> Self {
        Self::Text {
            text: value,
            prompt_cache_breakpoint: None,
        }
    }
}

impl From<&str> for ContentItem {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

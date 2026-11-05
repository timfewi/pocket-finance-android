//! Public no-model implementation. A category suggestion never authorizes a save.
//! This crate has no private dependency, network client, model, or receipt logging.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CategoryChoice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CategorySuggestion {
    pub category_id: Option<String>,
    pub status: SuggestionStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    Suggested,
    Unavailable,
    Failed,
}

/// Always unavailable in public builds. Manual editing and saving remain usable.
pub async fn suggest_category(
    _ocr_text: &str,
    _categories: &[CategoryChoice],
) -> CategorySuggestion {
    CategorySuggestion {
        category_id: None,
        status: SuggestionStatus::Unavailable,
    }
}

/// Diagnostics/tests only; do not display implementation choices in the main UI.
pub fn provider_name() -> &'static str {
    "public-no-model"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_is_unavailable_even_with_valid_receipt_and_choices() {
        let result = suggest_category(
            "SYNTHETIC MARKET\nBread\nTOTAL EUR 3,50",
            &[CategoryChoice {
                id: "groceries".into(),
                label: "Groceries".into(),
            }],
        )
        .await;
        assert_eq!(
            result,
            CategorySuggestion {
                category_id: None,
                status: SuggestionStatus::Unavailable
            }
        );
    }

    #[test]
    fn wire_contract_uses_camel_case_and_snake_case_status() {
        let result = CategorySuggestion {
            category_id: Some("groceries".into()),
            status: SuggestionStatus::Suggested,
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({"categoryId":"groceries","status":"suggested"})
        );
        assert_eq!(
            serde_json::to_value(SuggestionStatus::Unavailable).unwrap(),
            "unavailable"
        );
        assert_eq!(
            serde_json::to_value(SuggestionStatus::Failed).unwrap(),
            "failed"
        );
    }

    #[test]
    fn provider_can_be_identified_without_a_model() {
        assert_eq!(provider_name(), "public-no-model");
    }
}

use asset_common::{AssetData, AssetHandle, AssetRef};
use serde::Serialize;
use serde_json::json;

pub struct AssetJsonSchema;

impl AssetJsonSchema {
    pub fn create_schema<'a>(asset_refs: impl Iterator<Item = &'a AssetRef>) -> String {
        let one_of_values: Vec<_> = asset_refs
            .map(|asset_ref| OneOfValue {
                const_value: asset_ref.to_string(),
            })
            .collect();

        let schema_string = json!({
                    "$schema": "https://json-schema.org/draft-07/schema",
                    "$id": "https://example.com/assets.schema.json",
                    "title": "Assets",
                    "description": "All assets that can be loaded by the game",
                    "type": "object",
                    "properties": {
                        "$schema": {
                            "type": "string",
                        }
                    },
                    "patternProperties": {
                        "^([^$])(.+)$": {
                        "type": "string",
                        "oneOf": one_of_values
                        }
                    },
                    "additionalProperties": false
        });

        schema_string.to_string()
    }
}

#[derive(Debug, Serialize)]
struct OneOfValue {
    #[serde(rename = "const")]
    const_value: String,
}

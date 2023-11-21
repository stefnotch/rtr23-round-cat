use asset_common::AssetRef;
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

impl<'de, T: AssetData> de::Deserialize<'de> for AssetHandle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(AssetHandleVisitor::<T> {
            _marker: std::marker::PhantomData,
        })
    }
}

struct AssetHandleVisitor<T: AssetData> {
    _marker: std::marker::PhantomData<T>,
}
impl<'de, T: AssetData> de::Visitor<'de> for AssetHandleVisitor<T> {
    type Value = AssetHandle<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A string representing an asset reference")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(AssetHandle {
            key: AssetRef::new(v.split('/').map(|s| s.to_string()).collect()),
            _marker: std::marker::PhantomData,
        })
    }
}

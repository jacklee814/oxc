use std::path::PathBuf;

use schemars::{JsonSchema, r#gen::SchemaGenerator, schema::Schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// External plugin entry containing the plugin specifier and optional custom name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalPluginEntry {
    /// Directory containing the config file that specified this plugin
    pub config_dir: PathBuf,
    /// Plugin specifier (path, package name, or URL)
    pub specifier: String,
    /// Optional custom name/alias for the plugin
    pub name: Option<String>,
}

impl JsonSchema for ExternalPluginEntry {
    fn schema_name() -> String {
        "ExternalPluginEntry".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        use schemars::schema::{
            InstanceType, Metadata, ObjectValidation, SchemaObject, SubschemaValidation,
        };

        // Schema represents: string | { name: string, specifier: string }
        let string_schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            metadata: Some(Box::new(Metadata {
                description: Some("Path or package name of the plugin".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        let mut object_properties = schemars::Map::new();
        object_properties.insert(
            "name".to_string(),
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                metadata: Some(Box::new(Metadata {
                    description: Some("Custom name/alias for the plugin".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
        );
        object_properties.insert(
            "specifier".to_string(),
            SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                metadata: Some(Box::new(Metadata {
                    description: Some("Path or package name of the plugin".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            }
            .into(),
        );

        let object_schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            metadata: Some(Box::new(Metadata {
                description: Some("Plugin with custom name/alias".to_string()),
                ..Default::default()
            })),
            object: Some(Box::new(ObjectValidation {
                properties: object_properties,
                required: vec!["name".to_string(), "specifier".to_string()].into_iter().collect(),
                additional_properties: Some(Box::new(Schema::Bool(false))),
                ..Default::default()
            })),
            ..Default::default()
        };

        SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                any_of: Some(vec![string_schema.into(), object_schema.into()]),
                ..Default::default()
            })),
            ..Default::default()
        }
        .into()
    }
}

impl<'de> Deserialize<'de> for ExternalPluginEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct PluginObject {
            name: String,
            specifier: String,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PluginEntry {
            String(String),
            Object(PluginObject),
        }

        let entry = PluginEntry::deserialize(deserializer)?;

        Ok(match entry {
            PluginEntry::String(specifier) => {
                ExternalPluginEntry { config_dir: PathBuf::default(), specifier, name: None }
            }
            PluginEntry::Object(obj) => ExternalPluginEntry {
                config_dir: PathBuf::default(),
                specifier: obj.specifier,
                name: Some(obj.name),
            },
        })
    }
}

impl Serialize for ExternalPluginEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum PluginEntry<'a> {
            String(&'a str),
            Object { name: &'a str, specifier: &'a str },
        }

        if let Some(ref alias_name) = self.name {
            // Serialize as object: { "name": "alias", "specifier": "./plugin.ts" }
            PluginEntry::Object { name: alias_name.as_str(), specifier: self.specifier.as_str() }
                .serialize(serializer)
        } else {
            // Serialize as string: "./plugin.ts"
            PluginEntry::String(&self.specifier).serialize(serializer)
        }
    }
}

#[cfg(test)]
mod test {
    use rustc_hash::FxHashSet;

    use super::*;

    #[test]
    fn test_deserialize() {
        // Mixed formats
        let json = serde_json::json!([
            "./plugin.ts",
            { "name": "custom", "specifier": "./plugin2.ts" }
        ]);
        let plugins: Option<FxHashSet<ExternalPluginEntry>> = serde_json::from_value(json).unwrap();
        let plugins = plugins.unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins.iter().filter(|e| e.name.is_some()).count(), 1);

        // Null
        let json = serde_json::Value::Null;
        let plugins: Option<FxHashSet<ExternalPluginEntry>> = serde_json::from_value(json).unwrap();
        assert!(plugins.is_none());

        // Empty array
        let json = serde_json::json!([]);
        let plugins: Option<FxHashSet<ExternalPluginEntry>> = serde_json::from_value(json).unwrap();
        assert_eq!(plugins.unwrap().len(), 0);
    }

    #[test]
    fn test_deserialize_rejects_invalid() {
        // Unknown fields
        assert!(
            serde_json::from_value::<Option<FxHashSet<ExternalPluginEntry>>>(serde_json::json!([
                { "name": "x", "specifier": "y", "extra": "z" }
            ]))
            .is_err()
        );

        // Missing required fields
        assert!(
            serde_json::from_value::<Option<FxHashSet<ExternalPluginEntry>>>(
                serde_json::json!([{ "name": "x" }])
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<Option<FxHashSet<ExternalPluginEntry>>>(
                serde_json::json!([{ "specifier": "x" }])
            )
            .is_err()
        );
    }

    #[test]
    fn test_serialize() {
        let mut plugins = FxHashSet::default();
        plugins.insert(ExternalPluginEntry {
            config_dir: PathBuf::default(),
            specifier: "./plugin.ts".to_string(),
            name: None,
        });
        plugins.insert(ExternalPluginEntry {
            config_dir: PathBuf::default(),
            specifier: "./plugin2.ts".to_string(),
            name: Some("custom".to_string()),
        });

        let json = serde_json::to_value(&Some(plugins)).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Null
        let json = serde_json::to_value(&None::<FxHashSet<ExternalPluginEntry>>).unwrap();
        assert!(json.is_null());
    }
}

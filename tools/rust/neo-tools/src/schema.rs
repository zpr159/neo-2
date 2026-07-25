use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use neo_core::error::{NeoError, NeoResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    pub name: String,
    pub property_type: String,
    pub required: bool,
    pub description: String,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolSchema {
    pub properties: Vec<SchemaProperty>,
    pub required_properties: Vec<String>,
}

impl ToolSchema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_property(&mut self, prop: SchemaProperty) {
        if prop.required {
            self.required_properties.push(prop.name.clone());
        }
        self.properties.push(prop);
    }

    pub fn validate(&self, params: &Value) -> NeoResult<()> {
        let obj = params
            .as_object()
            .ok_or_else(|| NeoError::InvalidInput("Parameters must be a JSON object".into()))?;

        for req in &self.required_properties {
            if !obj.contains_key(req) {
                return Err(NeoError::InvalidInput(format!(
                    "Missing required parameter: {}",
                    req
                )));
            }
        }

        for prop in &self.properties {
            if let Some(value) = obj.get(&prop.name) {
                let type_ok = match prop.property_type.as_str() {
                    "string" => value.is_string(),
                    "number" => value.is_number(),
                    "boolean" => value.is_boolean(),
                    "array" => value.is_array(),
                    "object" => value.is_object(),
                    _ => true,
                };
                if !type_ok {
                    return Err(NeoError::InvalidInput(format!(
                        "Parameter '{}' expected type '{}', got {}",
                        prop.name,
                        prop.property_type,
                        match value {
                            Value::Null => "null",
                            Value::Bool(_) => "boolean",
                            Value::Number(_) => "number",
                            Value::String(_) => "string",
                            Value::Array(_) => "array",
                            Value::Object(_) => "object",
                        }
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn to_json_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        for prop in &self.properties {
            let mut schema = json!({
                "type": prop.property_type,
                "description": prop.description,
            });
            if let Some(ref default) = prop.default {
                schema["default"] = default.clone();
            }
            properties.insert(prop.name.clone(), schema);
        }

        json!({
            "type": "object",
            "properties": properties,
            "required": self.required_properties,
        })
    }
}

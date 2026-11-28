use serde_json::{Map, Value};

pub fn parse(text: &str) -> Result<Value, String> {
    let doc: serde_yaml::Value =
        serde_yaml::from_str(text).map_err(|e| first_line(&e.to_string()))?;
    Ok(convert(doc))
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("yaml parse error").to_string()
}

fn convert(v: serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s),
        serde_yaml::Value::Sequence(items) => {
            Value::Array(items.into_iter().map(convert).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(key_string(k), convert(v));
            }
            Value::Object(out)
        }
        serde_yaml::Value::Tagged(t) => convert(t.value),
    }
}

fn key_string(k: serde_yaml::Value) -> String {
    match k {
        serde_yaml::Value::String(s) => s,
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        other => serde_yaml::to_string(&other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

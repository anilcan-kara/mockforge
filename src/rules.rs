use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RuleSource {
    Param,
    Query,
    Header,
    Body,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleOp {
    Equals,
    NotEquals,
    Contains,
    Exists,
    NotExists,
}

#[derive(Debug, Clone)]
pub struct RuleCondition {
    pub source: RuleSource,
    pub key: String,
    pub op: RuleOp,
    pub value: Option<String>,
}

impl RuleCondition {
    pub fn parse(cond_str: &str) -> Option<Self> {
        let cond_str = cond_str.trim();

        // Check !exists / exists first
        if cond_str.ends_with("!exists") {
            let left = cond_str[..cond_str.len() - 7].trim();
            let (source, key) = parse_left(left)?;
            return Some(RuleCondition {
                source,
                key,
                op: RuleOp::NotExists,
                value: None,
            });
        }
        if cond_str.ends_with("exists") {
            let left = cond_str[..cond_str.len() - 6].trim();
            let (source, key) = parse_left(left)?;
            return Some(RuleCondition {
                source,
                key,
                op: RuleOp::Exists,
                value: None,
            });
        }

        // Check binary operators
        let ops = [
            ("==", RuleOp::Equals),
            ("!=", RuleOp::NotEquals),
            (" contains ", RuleOp::Contains),
        ];
        for (op_str, op) in ops {
            if let Some(pos) = cond_str.find(op_str) {
                let left = cond_str[..pos].trim();
                let right = cond_str[pos + op_str.len()..].trim();
                let (source, key) = parse_left(left)?;
                let val = strip_quotes(right);
                return Some(RuleCondition {
                    source,
                    key,
                    op,
                    value: Some(val.to_string()),
                });
            }
        }
        None
    }

    pub fn evaluate(
        &self,
        path_params: &HashMap<String, String>,
        query_params: &HashMap<String, String>,
        headers: &HashMap<String, String>,
        body: Option<&Value>,
    ) -> bool {
        let actual_value = match self.source {
            RuleSource::Param => path_params.get(&self.key).cloned(),
            RuleSource::Query => query_params.get(&self.key).cloned(),
            RuleSource::Header => headers.get(&self.key.to_lowercase()).cloned(),
            RuleSource::Body => body.and_then(|b| get_json_value(b, &self.key)),
        };

        match self.op {
            RuleOp::Exists => actual_value.is_some(),
            RuleOp::NotExists => actual_value.is_none(),
            RuleOp::Equals => {
                if let Some(val) = actual_value {
                    if let Some(ref expected) = self.value {
                        val == *expected
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            RuleOp::NotEquals => {
                if let Some(val) = actual_value {
                    if let Some(ref expected) = self.value {
                        val != *expected
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            RuleOp::Contains => {
                if let Some(val) = actual_value {
                    if let Some(ref expected) = self.value {
                        val.contains(expected)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

fn parse_left(left: &str) -> Option<(RuleSource, String)> {
    let parts: Vec<&str> = left.splitn(2, '.').collect();
    if parts.len() < 2 {
        return None;
    }
    let source = match parts[0] {
        "params" => RuleSource::Param,
        "query" => RuleSource::Query,
        "headers" => RuleSource::Header,
        "body" => RuleSource::Body,
        _ => return None,
    };
    Some((source, parts[1].to_string()))
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() >= 2 {
            &s[1..s.len() - 1]
        } else {
            s
        }
    } else {
        s
    }
}

fn get_json_value(val: &Value, key_path: &str) -> Option<String> {
    let mut current = val;
    for part in key_path.split('.') {
        match current {
            Value::Object(map) => {
                if let Some(next) = map.get(part) {
                    current = next;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
    match current {
        Value::Null => None,
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => Some(current.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rules() {
        let cond = RuleCondition::parse("params.id == '404'").unwrap();
        assert_eq!(cond.source, RuleSource::Param);
        assert_eq!(cond.key, "id");
        assert_eq!(cond.op, RuleOp::Equals);
        assert_eq!(cond.value.unwrap(), "404");

        let cond = RuleCondition::parse("headers.x-api-key != \"secret\"").unwrap();
        assert_eq!(cond.source, RuleSource::Header);
        assert_eq!(cond.key, "x-api-key");
        assert_eq!(cond.op, RuleOp::NotEquals);
        assert_eq!(cond.value.unwrap(), "secret");

        let cond = RuleCondition::parse("query.search contains admin").unwrap();
        assert_eq!(cond.source, RuleSource::Query);
        assert_eq!(cond.key, "search");
        assert_eq!(cond.op, RuleOp::Contains);
        assert_eq!(cond.value.unwrap(), "admin");

        let cond = RuleCondition::parse("body.user.role exists").unwrap();
        assert_eq!(cond.source, RuleSource::Body);
        assert_eq!(cond.key, "user.role");
        assert_eq!(cond.op, RuleOp::Exists);
        assert_eq!(cond.value, None);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedSecret {
    pub key: String,
    pub placeholder: String,
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedInput {
    pub text: String,
    pub secrets: Vec<DetectedSecret>,
}

#[derive(Debug, Clone)]
struct Candidate {
    start: usize,
    end: usize,
    value: String,
    label: String,
}

pub fn redact_secrets(input: &str) -> RedactedInput {
    let mut candidates = Vec::new();

    collect_pem_blocks(input, &mut candidates);
    collect_bearer_tokens(input, &mut candidates);
    collect_labelled_values(input, &mut candidates);
    collect_connection_url_credentials(input, &mut candidates);
    collect_prefixed_tokens(input, &mut candidates);
    collect_jwt_tokens(input, &mut candidates);

    candidates.sort_by_key(|candidate| candidate.start);
    let mut selected = Vec::new();
    for candidate in candidates {
        if is_existing_placeholder(input, candidate.start, candidate.end) {
            continue;
        }
        if candidate.value.trim().is_empty() {
            continue;
        }
        if selected.iter().any(|existing: &Candidate| {
            ranges_overlap(existing.start, existing.end, candidate.start, candidate.end)
        }) {
            continue;
        }
        selected.push(candidate);
    }

    let mut text = String::with_capacity(input.len());
    let mut cursor = 0;
    let mut secrets = Vec::with_capacity(selected.len());

    for (index, candidate) in selected.iter().enumerate() {
        let key = format!("pending_{}", index + 1);
        let placeholder = format!("{{{{secret:{key}}}}}");
        text.push_str(&input[cursor..candidate.start]);
        text.push_str(&placeholder);
        cursor = candidate.end;
        secrets.push(DetectedSecret {
            key,
            placeholder,
            value: candidate.value.clone(),
            label: candidate.label.clone(),
        });
    }
    text.push_str(&input[cursor..]);

    RedactedInput { text, secrets }
}

fn collect_pem_blocks(input: &str, candidates: &mut Vec<Candidate>) {
    let mut cursor = 0;
    while let Some(relative_start) = input[cursor..].find("-----BEGIN ") {
        let start = cursor + relative_start;
        let Some(relative_end_marker) = input[start..].find("-----END ") else {
            break;
        };
        let end_marker_start = start + relative_end_marker;
        let Some(relative_end) = input[end_marker_start..].find("-----") else {
            break;
        };
        let end = end_marker_start + relative_end + "-----".len();
        let value = input[start..end].to_string();
        candidates.push(Candidate {
            start,
            end,
            value,
            label: "privateKey".to_string(),
        });
        cursor = end;
    }
}

fn collect_bearer_tokens(input: &str, candidates: &mut Vec<Candidate>) {
    let lower = input.to_lowercase();
    let mut cursor = 0;
    while let Some(relative_start) = lower[cursor..].find("bearer ") {
        let token_start = cursor + relative_start + "bearer ".len();
        let token_end = consume_secret_value(input, token_start);
        if token_end > token_start {
            let value = trim_wrappers(&input[token_start..token_end]);
            if is_probably_secret_value(&value) {
                candidates.push(Candidate {
                    start: token_start,
                    end: token_start + value.len(),
                    value,
                    label: "bearerToken".to_string(),
                });
            }
        }
        cursor = token_start.saturating_add(1);
    }
}

fn collect_labelled_values(input: &str, candidates: &mut Vec<Candidate>) {
    let labels = [
        "api key",
        "access key",
        "private key",
        "password",
        "passwd",
        "token",
        "secret",
        "apikey",
        "pwd",
        "密码",
        "口令",
        "密钥",
        "令牌",
        "私钥",
        "访问密钥",
    ];
    let lower = input.to_lowercase();

    for label in labels {
        let search_space = if label.is_ascii() {
            lower.as_str()
        } else {
            input
        };
        let mut cursor = 0;
        while let Some(relative_start) = search_space[cursor..].find(label) {
            let label_start = cursor + relative_start;
            let label_end = label_start + label.len();
            let Some(value_start) = skip_label_separator(input, label_end) else {
                cursor = label_end;
                continue;
            };
            let value_end = consume_secret_value(input, value_start);
            if value_end > value_start {
                let raw_value = &input[value_start..value_end];
                let value = trim_wrappers(raw_value);
                if is_probably_secret_value(&value) {
                    candidates.push(Candidate {
                        start: value_start,
                        end: value_start + value.len(),
                        value,
                        label: label.to_string(),
                    });
                }
            }
            cursor = label_end;
        }
    }
}

fn collect_prefixed_tokens(input: &str, candidates: &mut Vec<Candidate>) {
    let prefixes = ["sk-", "ghp_", "gho_", "github_pat_", "xoxb-", "AKIA"];
    for prefix in prefixes {
        let mut cursor = 0;
        while let Some(relative_start) = input[cursor..].find(prefix) {
            let start = cursor + relative_start;
            let end = consume_secret_value(input, start);
            if end > start {
                let value = trim_wrappers(&input[start..end]);
                if value.len() >= prefix.len() + 6 {
                    candidates.push(Candidate {
                        start,
                        end: start + value.len(),
                        value,
                        label: prefix.to_string(),
                    });
                }
            }
            cursor = start + prefix.len();
        }
    }
}

fn collect_connection_url_credentials(input: &str, candidates: &mut Vec<Candidate>) {
    let schemes = [
        "postgresql://",
        "postgres://",
        "mysql://",
        "mongodb://",
        "redis://",
        "jdbc:postgresql://",
        "jdbc:mysql://",
    ];
    let lower = input.to_lowercase();
    for scheme in schemes {
        let mut cursor = 0;
        while let Some(relative_start) = lower[cursor..].find(scheme) {
            let scheme_start = cursor + relative_start;
            let credentials_start = scheme_start + scheme.len();
            let segment_end = input[credentials_start..]
                .char_indices()
                .find_map(|(offset, ch)| {
                    is_value_terminator(ch).then_some(credentials_start + offset)
                })
                .unwrap_or(input.len());
            let segment = &input[credentials_start..segment_end];
            let Some(at_offset) = segment.find('@') else {
                cursor = credentials_start.saturating_add(1);
                continue;
            };
            let credentials_end = credentials_start + at_offset;
            let value = input[credentials_start..credentials_end].to_string();
            if value.contains(':') && is_probably_secret_value(&value) {
                candidates.push(Candidate {
                    start: credentials_start,
                    end: credentials_end,
                    value,
                    label: "connectionUrlCredentials".to_string(),
                });
            }
            cursor = credentials_end.saturating_add(1);
        }
    }
}

fn collect_jwt_tokens(input: &str, candidates: &mut Vec<Candidate>) {
    let mut token_start = None;
    for (index, ch) in input.char_indices() {
        if is_token_char(ch) {
            token_start.get_or_insert(index);
            continue;
        }

        if let Some(start) = token_start.take() {
            maybe_push_jwt(input, start, index, candidates);
        }
    }

    if let Some(start) = token_start {
        maybe_push_jwt(input, start, input.len(), candidates);
    }
}

fn maybe_push_jwt(input: &str, start: usize, end: usize, candidates: &mut Vec<Candidate>) {
    let token = &input[start..end];
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3
        && parts.iter().all(|part| part.len() >= 8)
        && token.len() >= 30
        && token.chars().all(is_token_char)
    {
        candidates.push(Candidate {
            start,
            end,
            value: token.to_string(),
            label: "jwt".to_string(),
        });
    }
}

fn skip_label_separator(input: &str, start: usize) -> Option<usize> {
    let mut cursor = start;
    let mut saw_separator = false;

    while cursor < input.len() {
        let ch = input[cursor..].chars().next()?;
        if ch.is_whitespace() || matches!(ch, ':' | '：' | '=' | '-' | '是' | '为') {
            saw_separator = true;
            cursor += ch.len_utf8();
            continue;
        }
        break;
    }

    saw_separator.then_some(cursor)
}

fn consume_secret_value(input: &str, start: usize) -> usize {
    let mut end = start;
    for (offset, ch) in input[start..].char_indices() {
        if is_value_terminator(ch) {
            break;
        }
        end = start + offset + ch.len_utf8();
    }
    end
}

fn is_value_terminator(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            ',' | '，'
                | ';'
                | '；'
                | '。'
                | '!'
                | '！'
                | '?'
                | '？'
                | ')'
                | '）'
                | ']'
                | '】'
                | '}'
                | '>'
                | '\''
                | '"'
                | '`'
        )
}

fn trim_wrappers(value: &str) -> String {
    value
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\''
                    | '`'
                    | ':'
                    | '：'
                    | ','
                    | '，'
                    | ';'
                    | '；'
                    | '。'
                    | ')'
                    | '）'
                    | ']'
                    | '】'
                    | '}'
            )
        })
        .to_string()
}

fn is_probably_secret_value(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 {
        return false;
    }
    if trimmed.starts_with("{{secret:") {
        return false;
    }
    !matches!(
        trimmed.to_lowercase().as_str(),
        "none" | "null" | "empty" | "未知" | "无" | "没有"
    )
}

fn is_existing_placeholder(input: &str, start: usize, end: usize) -> bool {
    let Some(before) = input.get(..start) else {
        return false;
    };
    let Some(after) = input.get(end..) else {
        return false;
    };

    before.rfind("{{secret:").is_some() && after.starts_with("}}")
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~' | '+' | '/' | '=')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_chinese_labelled_passwords() {
        let redacted =
            redact_secrets("服务器是 1.2.3.4，用户 root，密码 abc123。数据库密码：db888。");

        assert!(!redacted.text.contains("abc123"));
        assert!(!redacted.text.contains("db888"));
        assert!(redacted.text.contains("密码 {{secret:pending_1}}"));
        assert!(redacted.text.contains("数据库密码：{{secret:pending_2}}"));
        assert_eq!(redacted.secrets[0].value, "abc123");
        assert_eq!(redacted.secrets[1].value, "db888");
    }

    #[test]
    fn redacts_openai_and_github_style_tokens() {
        let redacted = redact_secrets(
            "OpenAI key sk-proj-abcdefghijklmnopqrstuvwxyz and GitHub ghp_abcdefghijklmnopqrstuvwxyz",
        );

        assert!(!redacted.text.contains("sk-proj-abcdefghijklmnopqrstuvwxyz"));
        assert!(!redacted.text.contains("ghp_abcdefghijklmnopqrstuvwxyz"));
        assert_eq!(redacted.secrets.len(), 2);
    }

    #[test]
    fn redacts_bearer_tokens() {
        let redacted = redact_secrets("Authorization: Bearer abc.def.ghi123456789");

        assert_eq!(redacted.text, "Authorization: Bearer {{secret:pending_1}}");
        assert_eq!(redacted.secrets[0].value, "abc.def.ghi123456789");
    }

    #[test]
    fn redacts_connection_url_credentials() {
        let redacted = redact_secrets(
            "连接 postgresql://user:secret@db.internal:5432/payments 以及 redis://:pass@cache:6379",
        );

        assert!(!redacted.text.contains("user:secret"));
        assert!(!redacted.text.contains(":pass@"));
        assert!(redacted
            .text
            .contains("postgresql://{{secret:pending_1}}@db.internal:5432/payments"));
        assert!(redacted
            .text
            .contains("redis://{{secret:pending_2}}@cache:6379"));
        assert_eq!(redacted.secrets.len(), 2);
    }

    #[test]
    fn redacts_private_key_blocks() {
        let input = "key:\n-----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY-----\nend";
        let redacted = redact_secrets(input);

        assert!(!redacted.text.contains("abc123"));
        assert!(redacted.text.contains("{{secret:pending_1}}"));
        assert_eq!(redacted.secrets[0].label, "privateKey");
    }

    #[test]
    fn keeps_existing_placeholders() {
        let redacted = redact_secrets("密码 {{secret:dbPassword}}");

        assert_eq!(redacted.text, "密码 {{secret:dbPassword}}");
        assert!(redacted.secrets.is_empty());
    }
}

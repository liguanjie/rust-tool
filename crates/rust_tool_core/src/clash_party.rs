use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct ClashPartyConfig {
    pub data_dir: String,
    pub api_url: String,
    pub api_secret: String,
    pub delay_test_url: String,
    pub delay_timeout_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartySubscription {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub path: String,
    pub active: bool,
    pub node_count: usize,
    pub group_count: usize,
    pub updated_at: String,
    pub used_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub expire_at: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyNode {
    pub name: String,
    pub display_name: String,
    pub node_type: String,
    pub server: String,
    pub port: Option<u16>,
    pub delay: Option<i64>,
    pub available: Option<bool>,
    pub check_message: String,
    pub active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyProxyGroup {
    pub name: String,
    pub display_name: String,
    pub group_type: String,
    pub selected: String,
    pub selected_display_name: String,
    pub nodes: Vec<ClashPartyNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyManagerState {
    pub data_dir: String,
    pub profile_index_path: String,
    pub api_url: String,
    pub active_subscription_id: String,
    pub subscriptions: Vec<ClashPartySubscription>,
    pub groups: Vec<ClashPartyProxyGroup>,
    pub api_available: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartySwitchResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyApiHealth {
    pub ok: bool,
    pub api_url: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClashPartyNodeCheckResult {
    pub ok: bool,
    pub available: bool,
    pub node_name: String,
    pub delay: Option<i64>,
    pub timeout_ms: u64,
    pub test_url: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ClashProfileIndex {
    #[serde(default)]
    pub items: Vec<ClashProfileItem>,
    #[serde(default)]
    pub current: String,
}

#[derive(Debug, Deserialize)]
pub struct ClashProfileItem {
    pub id: String,
    pub name: String,
    #[serde(default, rename = "type")]
    pub profile_type: String,
    #[serde(default)]
    pub updated: Option<i64>,
    #[serde(default)]
    pub extra: Option<ClashProfileExtra>,
}

#[derive(Debug, Deserialize)]
pub struct ClashProfileExtra {
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub upload: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub download: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub total: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub expire: Option<u64>,
}

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = YamlValue::deserialize(deserializer)?;
    Ok(match value {
        YamlValue::Null => None,
        YamlValue::Number(number) => number
            .as_u64()
            .or_else(|| number.as_i64().and_then(|value| u64::try_from(value).ok()))
            .or_else(|| number.as_f64().and_then(finite_f64_to_u64)),
        YamlValue::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("nan") {
                None
            } else {
                trimmed
                    .parse::<u64>()
                    .ok()
                    .or_else(|| trimmed.parse::<f64>().ok().and_then(finite_f64_to_u64))
            }
        }
        _ => None,
    })
}

fn finite_f64_to_u64(value: f64) -> Option<u64> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u64::MAX as f64 {
        Some(value as u64)
    } else {
        None
    }
}

pub fn default_clash_party_api_url() -> String {
    "http://127.0.0.1:9998".to_string()
}

pub fn default_clash_party_delay_test_url() -> String {
    "https://www.gstatic.com/generate_204".to_string()
}

pub fn default_clash_party_delay_timeout_ms() -> u64 {
    5000
}

pub fn detect_clash_party_data_dir() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(app_data) = env::var("APPDATA") {
        candidates.extend([
            format!(r"{app_data}\mihomo-party"),
            format!(r"{app_data}\clash-party"),
            format!(r"{app_data}\clashmi"),
        ]);
    }
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates.extend([
            format!(r"{local_app_data}\mihomo-party"),
            format!(r"{local_app_data}\clash-party"),
            format!(r"{local_app_data}\clashmi"),
        ]);
    }

    candidates
        .iter()
        .map(String::as_str)
        .find(|path| Path::new(path).join("profile.yaml").exists())
        .map(ToOwned::to_owned)
}

pub fn normalized_clash_party_api_url(config: &ClashPartyConfig) -> String {
    let url = config.api_url.trim();
    if url.is_empty() {
        default_clash_party_api_url()
    } else {
        url.trim_end_matches('/').to_string()
    }
}

pub fn normalized_clash_party_delay_test_url(config: &ClashPartyConfig) -> String {
    let url = config.delay_test_url.trim();
    if url.is_empty() {
        default_clash_party_delay_test_url()
    } else {
        url.to_string()
    }
}

pub fn normalized_clash_party_delay_timeout_ms(config: &ClashPartyConfig) -> u64 {
    if config.delay_timeout_ms == 0 {
        default_clash_party_delay_timeout_ms()
    } else {
        config.delay_timeout_ms
    }
}

pub fn get_clash_party_manager_state(
    config: &ClashPartyConfig,
) -> Result<ClashPartyManagerState, String> {
    let mut data_error = String::new();
    let (data_dir, profile_index_path, active_subscription_id, subscriptions) =
        match read_clash_subscriptions(config) {
            Ok(profile_data) => profile_data,
            Err(error) => {
                data_error = error;
                (String::new(), String::new(), String::new(), Vec::new())
            }
        };

    let runtime_result = get_clash_party_runtime_groups(config);
    let (groups, api_available, message) = match runtime_result {
        Ok(groups) => {
            let message = if data_error.is_empty() {
                if groups.is_empty() {
                    "已读取订阅；Mihomo API 可访问，但未返回可切换代理组".to_string()
                } else {
                    format!(
                        "已读取 {} 个订阅和 {} 个运行时代理组",
                        subscriptions.len(),
                        groups.len()
                    )
                }
            } else if groups.is_empty() {
                format!("Mihomo API 已连接；{data_error}")
            } else {
                format!(
                    "Mihomo API 已连接，读取到 {} 个运行时代理组；{data_error}",
                    groups.len()
                )
            };
            (groups, true, message)
        }
        Err(runtime_error) => {
            if data_error.is_empty() {
                (
                    Vec::new(),
                    false,
                    format!(
                        "已读取 {} 个订阅；Mihomo API 未连接，请确认 Clash Party 正在运行，并且 API 地址/端口已开放。",
                        subscriptions.len()
                    ),
                )
            } else {
                return Err(format!("{data_error}；Mihomo API 未连接: {runtime_error}"));
            }
        }
    };

    Ok(ClashPartyManagerState {
        data_dir,
        profile_index_path,
        api_url: normalized_clash_party_api_url(config),
        active_subscription_id,
        subscriptions,
        groups,
        api_available,
        message,
    })
}

pub fn check_clash_party_api(config: &ClashPartyConfig) -> ClashPartyApiHealth {
    let api_url = normalized_clash_party_api_url(config);
    match call_clash_party_api(config, "GET", "/proxies", None) {
        Ok(_) => ClashPartyApiHealth {
            ok: true,
            api_url,
            message: "Mihomo API 已连接".to_string(),
        },
        Err(error) => ClashPartyApiHealth {
            ok: false,
            api_url,
            message: error,
        },
    }
}

pub fn switch_clash_party_subscription(
    config: &ClashPartyConfig,
    subscription_id: &str,
) -> Result<ClashPartySwitchResult, String> {
    let data_dir = resolve_clash_party_data_dir(config)?;
    let profile_index_path = data_dir.join("profile.yaml");
    let profile_index = read_clash_profile_index(&profile_index_path)?;
    let exists = profile_index
        .items
        .iter()
        .any(|item| item.id == subscription_id);
    if !exists {
        return Err("订阅不存在，请先刷新订阅列表".to_string());
    }

    let profile_path = clash_profile_path(&data_dir, subscription_id);
    if !profile_path.exists() {
        return Err(format!("订阅配置文件不存在: {}", profile_path.display()));
    }

    let body = serde_json::json!({
        "path": profile_path.display().to_string(),
    });
    let response = call_clash_party_api(config, "PUT", "/configs", Some(body))?;
    Ok(ClashPartySwitchResult {
        ok: true,
        message: if response.is_empty() {
            format!("已请求切换订阅: {subscription_id}")
        } else {
            response
        },
    })
}

pub fn switch_clash_party_node(
    config: &ClashPartyConfig,
    group_name: &str,
    node_name: &str,
) -> Result<ClashPartySwitchResult, String> {
    if group_name.trim().is_empty() || node_name.trim().is_empty() {
        return Err("代理组和节点名称不能为空".to_string());
    }

    let check = check_clash_party_node(config, node_name)?;
    if !check.available {
        return Err(format!("节点检测未通过，已取消切换：{}", check.message));
    }

    let path = format!("/proxies/{}", encode_url_path_segment(group_name));
    let body = serde_json::json!({
        "name": node_name,
    });
    let response = call_clash_party_api(config, "PUT", &path, Some(body))?;
    Ok(ClashPartySwitchResult {
        ok: true,
        message: if response.is_empty() {
            format!("已请求将 {group_name} 切换到 {node_name}")
        } else {
            response
        },
    })
}

pub fn check_clash_party_node(
    config: &ClashPartyConfig,
    node_name: &str,
) -> Result<ClashPartyNodeCheckResult, String> {
    let node_name = node_name.trim();
    if node_name.is_empty() {
        return Err("节点名称不能为空".to_string());
    }

    let timeout_ms = normalized_clash_party_delay_timeout_ms(config);
    let test_url = normalized_clash_party_delay_test_url(config);
    let path = format!(
        "/proxies/{}/delay?timeout={}&url={}",
        encode_url_path_segment(node_name),
        timeout_ms,
        encode_url_query_value(&test_url)
    );

    match call_clash_party_api(config, "GET", &path, None) {
        Ok(text) => {
            let value: JsonValue = serde_json::from_str(&text)
                .map_err(|error| format!("解析节点检测结果失败: {error}"))?;
            let delay = value.get("delay").and_then(JsonValue::as_i64);
            let available = node_available_from_delay(delay, timeout_ms).unwrap_or(false);
            Ok(ClashPartyNodeCheckResult {
                ok: available,
                available,
                node_name: node_name.to_string(),
                delay,
                timeout_ms,
                test_url,
                message: node_check_message(delay, timeout_ms),
            })
        }
        Err(error) => Ok(ClashPartyNodeCheckResult {
            ok: false,
            available: false,
            node_name: node_name.to_string(),
            delay: None,
            timeout_ms,
            test_url,
            message: format!("节点检测失败或已超时: {error}"),
        }),
    }
}

fn read_clash_subscriptions(
    config: &ClashPartyConfig,
) -> Result<(String, String, String, Vec<ClashPartySubscription>), String> {
    let data_dir = resolve_clash_party_data_dir(config)?;
    let profile_index_path = data_dir.join("profile.yaml");
    let profile_index = read_clash_profile_index(&profile_index_path)?;
    let subscriptions = build_clash_subscriptions(&data_dir, &profile_index);

    Ok((
        data_dir.display().to_string(),
        profile_index_path.display().to_string(),
        profile_index.current,
        subscriptions,
    ))
}

fn resolve_clash_party_data_dir(config: &ClashPartyConfig) -> Result<PathBuf, String> {
    let configured = config.data_dir.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.join("profile.yaml").exists() {
            return Ok(path);
        }
        return Err(format!(
            "Clash Party 数据目录无效，未找到 profile.yaml: {}",
            path.display()
        ));
    }

    detect_clash_party_data_dir()
        .map(PathBuf::from)
        .ok_or_else(|| "未找到 Clash Party 数据目录，请配置 mihomo-party 数据目录".to_string())
}

fn read_clash_profile_index(path: &Path) -> Result<ClashProfileIndex, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("读取订阅索引失败: {error}"))?;
    serde_yaml::from_str(&content).map_err(|error| format!("解析订阅索引失败: {error}"))
}

fn build_clash_subscriptions(
    data_dir: &Path,
    profile_index: &ClashProfileIndex,
) -> Vec<ClashPartySubscription> {
    profile_index
        .items
        .iter()
        .map(|item| {
            let path = clash_profile_path(data_dir, &item.id);
            let (node_count, group_count) = read_clash_profile_summary(&path).unwrap_or((0, 0));
            let (used_bytes, total_bytes, expire_at) = item
                .extra
                .as_ref()
                .map(|extra| {
                    (
                        extra
                            .upload
                            .unwrap_or(0)
                            .saturating_add(extra.download.unwrap_or(0)),
                        extra.total,
                        extra.expire,
                    )
                })
                .unwrap_or((0, None, None));
            ClashPartySubscription {
                id: item.id.clone(),
                name: item.name.clone(),
                profile_type: if item.profile_type.is_empty() {
                    "unknown".to_string()
                } else {
                    item.profile_type.clone()
                },
                path: path.display().to_string(),
                active: item.id == profile_index.current,
                node_count,
                group_count,
                updated_at: item
                    .updated
                    .map_or_else(String::new, |value| value.to_string()),
                used_bytes: (used_bytes > 0).then_some(used_bytes),
                total_bytes,
                expire_at,
            }
        })
        .collect()
}

fn clash_profile_path(data_dir: &Path, subscription_id: &str) -> PathBuf {
    data_dir
        .join("profiles")
        .join(format!("{}.yaml", sanitize_path_file_stem(subscription_id)))
}

fn read_clash_profile_summary(path: &Path) -> Result<(usize, usize), String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let yaml: YamlValue = serde_yaml::from_str(&content).map_err(|error| error.to_string())?;
    let node_count = yaml_sequence_len(&yaml, "proxies");
    let group_count = yaml_sequence_len(&yaml, "proxy-groups");
    Ok((node_count, group_count))
}

fn yaml_sequence_len(value: &YamlValue, key: &str) -> usize {
    value
        .get(key)
        .and_then(YamlValue::as_sequence)
        .map(Vec::len)
        .unwrap_or_default()
}

fn get_clash_party_runtime_groups(
    config: &ClashPartyConfig,
) -> Result<Vec<ClashPartyProxyGroup>, String> {
    let text = call_clash_party_api(config, "GET", "/proxies", None)?;
    let value: JsonValue = serde_json::from_str(&text)
        .map_err(|error| format!("解析 Mihomo 代理列表失败: {error}"))?;
    let proxies = value
        .get("proxies")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "Mihomo API 未返回 proxies 字段".to_string())?;
    let profile_metadata = read_current_clash_profile_metadata(config).unwrap_or_default();
    let timeout_ms = normalized_clash_party_delay_timeout_ms(config);

    let mut proxy_detail = HashMap::new();
    for (name, proxy) in proxies {
        proxy_detail.insert(name.clone(), proxy.clone());
    }

    let mut groups = Vec::new();
    for (name, proxy) in proxies {
        let Some(all) = proxy.get("all").and_then(JsonValue::as_array) else {
            continue;
        };
        if all.is_empty() {
            continue;
        }
        let selected = proxy
            .get("now")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string();
        let selected_display_name = clash_display_name(&selected, &profile_metadata.names);
        let group_type = proxy
            .get("type")
            .and_then(JsonValue::as_str)
            .unwrap_or("Selector")
            .to_string();
        let nodes = all
            .iter()
            .filter_map(JsonValue::as_str)
            .map(|node_name| {
                let detail = proxy_detail.get(node_name);
                let display_name = clash_display_name(node_name, &profile_metadata.names);
                let delay = read_proxy_delay(detail);
                let available = node_available_from_delay(delay, timeout_ms);
                ClashPartyNode {
                    name: node_name.to_string(),
                    display_name,
                    node_type: detail
                        .and_then(|value| value.get("type"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    server: detail
                        .and_then(|value| value.get("server"))
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    port: detail
                        .and_then(|value| value.get("port"))
                        .and_then(JsonValue::as_u64)
                        .and_then(|value| u16::try_from(value).ok()),
                    delay,
                    available,
                    check_message: available
                        .map(|_| node_check_message(delay, timeout_ms))
                        .unwrap_or_default(),
                    active: node_name == selected,
                }
            })
            .collect();
        groups.push(ClashPartyProxyGroup {
            name: name.clone(),
            display_name: clash_display_name(name, &profile_metadata.names),
            group_type,
            selected,
            selected_display_name,
            nodes,
        });
    }

    groups.sort_by(|left, right| {
        let left_order = profile_metadata
            .group_order
            .get(&left.name)
            .copied()
            .unwrap_or(usize::MAX);
        let right_order = profile_metadata
            .group_order
            .get(&right.name)
            .copied()
            .unwrap_or(usize::MAX);
        left_order.cmp(&right_order).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    Ok(groups)
}

fn read_proxy_delay(detail: Option<&JsonValue>) -> Option<i64> {
    detail
        .and_then(|value| value.get("history"))
        .and_then(JsonValue::as_array)
        .and_then(|history| history.last())
        .and_then(|item| item.get("delay"))
        .and_then(JsonValue::as_i64)
}

fn node_available_from_delay(delay: Option<i64>, timeout_ms: u64) -> Option<bool> {
    let delay = delay?;
    Some(delay >= 0 && delay <= timeout_ms as i64)
}

fn node_check_message(delay: Option<i64>, timeout_ms: u64) -> String {
    match node_available_from_delay(delay, timeout_ms) {
        Some(true) => format!("节点可用，延迟 {}ms", delay.unwrap_or_default()),
        Some(false) => format!("节点超时或不可用，阈值 {timeout_ms}ms"),
        None => "尚未检测".to_string(),
    }
}

#[derive(Debug, Default)]
struct ClashProfileMetadata {
    names: HashMap<String, String>,
    group_order: HashMap<String, usize>,
}

fn read_current_clash_profile_metadata(
    config: &ClashPartyConfig,
) -> Result<ClashProfileMetadata, String> {
    let data_dir = resolve_clash_party_data_dir(config)?;
    let profile_index = read_clash_profile_index(&data_dir.join("profile.yaml"))?;
    if profile_index.current.trim().is_empty() {
        return Ok(ClashProfileMetadata::default());
    }

    let profile_path = clash_profile_path(&data_dir, &profile_index.current);
    let content = fs::read_to_string(&profile_path)
        .map_err(|error| format!("读取当前订阅配置失败: {error}"))?;
    let yaml: YamlValue =
        serde_yaml::from_str(&content).map_err(|error| format!("解析当前订阅配置失败: {error}"))?;

    let mut metadata = ClashProfileMetadata::default();
    collect_clash_named_items(&yaml, "proxies", &mut metadata.names, None);
    collect_clash_named_items(
        &yaml,
        "proxy-groups",
        &mut metadata.names,
        Some(&mut metadata.group_order),
    );
    Ok(metadata)
}

fn collect_clash_named_items(
    yaml: &YamlValue,
    key: &str,
    names: &mut HashMap<String, String>,
    mut order: Option<&mut HashMap<String, usize>>,
) {
    let Some(items) = yaml.get(key).and_then(YamlValue::as_sequence) else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let Some(name) = item.get("name").and_then(YamlValue::as_str) else {
            continue;
        };
        names.insert(name.to_string(), name.to_string());
        if let Some(order) = order.as_deref_mut() {
            order.entry(name.to_string()).or_insert(index);
        }
        if let Some(mojibake) = utf8_mojibake_key(name) {
            names
                .entry(mojibake.clone())
                .or_insert_with(|| name.to_string());
            if let Some(order) = order.as_deref_mut() {
                order.entry(mojibake).or_insert(index);
            }
        }
    }
}

fn clash_display_name(raw_name: &str, profile_names: &HashMap<String, String>) -> String {
    profile_names
        .get(raw_name)
        .cloned()
        .or_else(|| repair_utf8_mojibake(raw_name))
        .unwrap_or_else(|| raw_name.to_string())
}

fn utf8_mojibake_key(value: &str) -> Option<String> {
    if value.is_ascii() {
        return None;
    }
    Some(
        value
            .as_bytes()
            .iter()
            .map(|byte| *byte as char)
            .collect::<String>(),
    )
}

fn repair_utf8_mojibake(value: &str) -> Option<String> {
    if !looks_like_utf8_mojibake(value) {
        return None;
    }
    let bytes: Vec<u8> = value
        .chars()
        .map(|character| u32::from(character))
        .collect::<Vec<_>>()
        .into_iter()
        .map(u8::try_from)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let repaired = String::from_utf8(bytes).ok()?;
    (repaired != value).then_some(repaired)
}

fn looks_like_utf8_mojibake(value: &str) -> bool {
    value.contains('脙')
        || value.contains('脗')
        || value.contains('盲')
        || value.contains('氓')
        || value.contains('忙')
        || value.contains('莽')
        || value.contains('猫')
        || value.contains('茅')
        || value.contains('茂')
}

fn call_clash_party_api(
    config: &ClashPartyConfig,
    method: &str,
    path: &str,
    body: Option<JsonValue>,
) -> Result<String, String> {
    let base_url = normalized_clash_party_api_url(config);
    let request = ClashHttpRequest::new(&base_url, method, path, body, config.api_secret.trim())?;
    send_clash_http_request(&request)
}

struct ClashHttpRequest {
    host: String,
    port: u16,
    path: String,
    method: String,
    body: String,
    secret: String,
}

impl ClashHttpRequest {
    fn new(
        base_url: &str,
        method: &str,
        path: &str,
        body: Option<JsonValue>,
        secret: &str,
    ) -> Result<Self, String> {
        let trimmed = base_url.trim().trim_end_matches('/');
        let without_scheme = trimmed.strip_prefix("http://").ok_or_else(|| {
            "Clash Party API 目前只支持本机 http:// 地址，请将 API 地址配置为 http://127.0.0.1:9998 这类格式。".to_string()
        })?;
        if without_scheme.contains('@') {
            return Err("Clash Party API 地址不支持用户名密码写法".to_string());
        }
        let authority = without_scheme
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .trim();
        let (host, port) = parse_http_authority(authority)?;
        let request_path = format!("/{}", path.trim_start_matches('/'));

        Ok(Self {
            host,
            port,
            path: request_path,
            method: method.to_ascii_uppercase(),
            body: body.map_or_else(String::new, |value| value.to_string()),
            secret: secret.to_string(),
        })
    }
}

fn parse_http_authority(authority: &str) -> Result<(String, u16), String> {
    if authority.is_empty() {
        return Err("Clash Party API 地址缺少主机名".to_string());
    }
    if let Some(stripped) = authority.strip_prefix('[') {
        let Some((host, rest)) = stripped.split_once(']') else {
            return Err("Clash Party API IPv6 地址格式不正确".to_string());
        };
        let port = rest
            .strip_prefix(':')
            .map(parse_http_port)
            .transpose()?
            .unwrap_or(80);
        return Ok((host.to_string(), port));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host, parse_http_port(port)?),
        Some(_) if authority.matches(':').count() > 1 => (authority, 80),
        _ => (authority, 80),
    };
    Ok((host.to_string(), port))
}

fn parse_http_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| "Clash Party API 端口不是有效数字".to_string())
}

fn send_clash_http_request(request: &ClashHttpRequest) -> Result<String, String> {
    let mut stream = TcpStream::connect((request.host.as_str(), request.port))
        .map_err(|error| format!("连接 Clash Party API 失败: {error}"))?;
    let timeout = Some(Duration::from_secs(6));
    stream
        .set_read_timeout(timeout)
        .map_err(|error| format!("设置 Clash Party API 读取超时失败: {error}"))?;
    stream
        .set_write_timeout(timeout)
        .map_err(|error| format!("设置 Clash Party API 写入超时失败: {error}"))?;

    let mut http = format!(
        "{} {} HTTP/1.1\r\nHost: {}:{}\r\nAccept: application/json\r\nConnection: close\r\n",
        request.method, request.path, request.host, request.port
    );
    if !request.secret.is_empty() {
        http.push_str("Authorization: Bearer ");
        http.push_str(&request.secret);
        http.push_str("\r\n");
    }
    if request.body.is_empty() {
        http.push_str("\r\n");
    } else {
        http.push_str("Content-Type: application/json; charset=utf-8\r\n");
        http.push_str(&format!(
            "Content-Length: {}\r\n\r\n{}",
            request.body.len(),
            request.body
        ));
    }

    stream
        .write_all(http.as_bytes())
        .map_err(|error| format!("发送 Clash Party API 请求失败: {error}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("读取 Clash Party API 响应失败: {error}"))?;
    parse_clash_http_response(&response)
}

fn parse_clash_http_response(response: &[u8]) -> Result<String, String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "Clash Party API 响应格式不完整".to_string())?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let mut lines = headers.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "Clash Party API 响应缺少状态行".to_string())?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "Clash Party API 响应状态码无效".to_string())?;
    let chunked = lines.clone().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value.to_ascii_lowercase().contains("chunked")
        })
    });
    let body_bytes = &response[header_end + 4..];
    let body = if chunked {
        decode_chunked_body(body_bytes)?
    } else {
        body_bytes.to_vec()
    };
    let text = String::from_utf8(body)
        .map_err(|_| "Clash Party API 响应不是有效 UTF-8，请检查 Mihomo API 输出编码".to_string())?
        .trim()
        .to_string();

    if (200..300).contains(&status_code) {
        Ok(text)
    } else if text.is_empty() {
        Err(format!("Clash Party API 请求失败：HTTP {status_code}"))
    } else {
        Err(format!(
            "Clash Party API 请求失败：HTTP {status_code}，{text}"
        ))
    }
}

fn decode_chunked_body(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    let mut index = 0;
    loop {
        let line_end = find_crlf(body, index)
            .ok_or_else(|| "Clash Party API chunked 响应格式不完整".to_string())?;
        let size_text = String::from_utf8_lossy(&body[index..line_end]);
        let size_hex = size_text.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| "Clash Party API chunked 响应块大小无效".to_string())?;
        index = line_end + 2;
        if size == 0 {
            break;
        }
        let chunk_end = index
            .checked_add(size)
            .ok_or_else(|| "Clash Party API chunked 响应过大".to_string())?;
        if chunk_end + 2 > body.len() {
            return Err("Clash Party API chunked 响应数据不完整".to_string());
        }
        decoded.extend_from_slice(&body[index..chunk_end]);
        index = chunk_end + 2;
    }
    Ok(decoded)
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|offset| start + offset)
}

fn encode_url_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn encode_url_query_value(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn sanitize_path_file_stem(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) && !character.is_control()
        })
        .collect()
}

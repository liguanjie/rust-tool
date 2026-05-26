use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use url::{ParseError, Url};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputMode {
    FullConfig,
    ProxyOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateMode {
    Minimal,
    Standard,
    FullRules,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConvertOptions {
    pub output_mode: OutputMode,
    pub template_mode: TemplateMode,
    pub proxy_name: Option<String>,
    pub direct_domains: Vec<String>,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            output_mode: OutputMode::FullConfig,
            template_mode: TemplateMode::Standard,
            proxy_name: None,
            direct_domains: Vec::new(),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ConvertError {
    #[error("please input a vless:// link")]
    EmptyInput,
    #[error("link must start with vless://")]
    InvalidScheme,
    #[error("missing UUID")]
    MissingUuid,
    #[error("missing server address")]
    MissingServer,
    #[error("invalid port")]
    InvalidPort,
    #[error("failed to serialize YAML: {0}")]
    YamlSerializeFailed(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct MihomoConfig {
    #[serde(rename = "mixed-port")]
    mixed_port: u16,
    #[serde(rename = "allow-lan")]
    allow_lan: bool,
    #[serde(rename = "bind-address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    bind_address: Option<String>,
    mode: String,
    #[serde(rename = "log-level")]
    log_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dns: Option<DnsConfig>,
    proxies: Vec<Proxy>,
    #[serde(rename = "proxy-groups")]
    proxy_groups: Vec<ProxyGroup>,
    #[serde(rename = "rule-providers")]
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_providers: Option<BTreeMap<String, RuleProvider>>,
    rules: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProxyGroup {
    name: String,
    #[serde(rename = "type")]
    group_type: String,
    proxies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tolerance: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct DnsConfig {
    enable: bool,
    ipv6: bool,
    enhanced_mode: String,
    fake_ip_range: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_algorithm: Option<String>,
    default_nameserver: Vec<String>,
    nameserver: Vec<String>,
    fallback: Vec<String>,
    fallback_filter: DnsFallbackFilter,
    respect_rules: bool,
    proxy_server_nameserver: Vec<String>,
    fake_ip_filter: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct DnsFallbackFilter {
    geoip: bool,
    geoip_code: String,
    geosite: Vec<String>,
    ipcidr: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct RuleProvider {
    #[serde(rename = "type")]
    provider_type: String,
    behavior: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Proxy {
    name: String,
    #[serde(rename = "type")]
    proxy_type: String,
    server: String,
    port: u16,
    uuid: String,
    udp: bool,
    tls: bool,
    network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    servername: Option<String>,
    #[serde(rename = "client-fingerprint")]
    #[serde(skip_serializing_if = "Option::is_none")]
    client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alpn: Option<Vec<String>>,
    #[serde(rename = "skip-cert-verify")]
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_cert_verify: Option<bool>,
    #[serde(rename = "reality-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    reality_opts: Option<RealityOptions>,
    #[serde(rename = "packet-encoding")]
    #[serde(skip_serializing_if = "Option::is_none")]
    packet_encoding: Option<String>,
    #[serde(rename = "ws-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    ws_opts: Option<WebSocketOptions>,
    #[serde(rename = "grpc-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    grpc_opts: Option<GrpcOptions>,
    #[serde(rename = "httpupgrade-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    httpupgrade_opts: Option<HttpUpgradeOptions>,
    #[serde(rename = "xhttp-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    xhttp_opts: Option<XHttpOptions>,
    #[serde(rename = "h2-opts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    h2_opts: Option<H2Options>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct RealityOptions {
    #[serde(rename = "public-key")]
    #[serde(skip_serializing_if = "Option::is_none")]
    public_key: Option<String>,
    #[serde(rename = "short-id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    short_id: Option<String>,
    #[serde(rename = "spider-x")]
    #[serde(skip_serializing_if = "Option::is_none")]
    spider_x: Option<String>,
}

#[derive(Debug, Serialize)]
struct WebSocketOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct GrpcOptions {
    #[serde(rename = "grpc-service-name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    grpc_service_name: Option<String>,
    #[serde(rename = "grpc-mode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    grpc_mode: Option<String>,
    #[serde(rename = "grpc-authority")]
    #[serde(skip_serializing_if = "Option::is_none")]
    grpc_authority: Option<String>,
}

#[derive(Debug, Serialize)]
struct HttpUpgradeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct XHttpOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
}

#[derive(Debug, Serialize)]
struct H2Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<Vec<String>>,
}

pub fn convert_vless_to_yaml(input: &str, options: ConvertOptions) -> Result<String, ConvertError> {
    let mut proxies = parse_vless_links(input)?;
    if proxies.len() == 1 {
        if let Some(proxy_name) = normalize_custom_name(options.proxy_name.as_deref()) {
            proxies[0].name = proxy_name;
        }
    }
    ensure_unique_proxy_names(&mut proxies);
    let output_mode = options.output_mode;
    let template_mode = options.template_mode;
    let direct_domains = options.direct_domains;
    let node_addresses = proxies
        .iter()
        .map(|proxy| format!("{}:{}", proxy.server, proxy.port))
        .collect::<Vec<_>>();

    let mut yaml = match output_mode {
        OutputMode::FullConfig => {
            serde_yaml::to_string(&build_config(proxies, template_mode, &direct_domains))
        }
        OutputMode::ProxyOnly => {
            if proxies.len() == 1 {
                serde_yaml::to_string(&proxies[0])
            } else {
                serde_yaml::to_string(&proxies)
            }
        }
    }
    .map_err(|error| ConvertError::YamlSerializeFailed(error.to_string()))?;
    yaml.insert_str(0, &format_node_addresses_comment(&node_addresses));

    Ok(yaml)
}

fn parse_vless_links(input: &str) -> Result<Vec<Proxy>, ConvertError> {
    let links = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if links.is_empty() {
        return Err(ConvertError::EmptyInput);
    }

    links.into_iter().map(parse_vless).collect()
}

fn parse_vless(input: &str) -> Result<Proxy, ConvertError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(ConvertError::EmptyInput);
    }

    if !raw.to_ascii_lowercase().starts_with("vless://") {
        return Err(ConvertError::InvalidScheme);
    }

    let url = Url::parse(raw).map_err(|error| match error {
        ParseError::InvalidPort => ConvertError::InvalidPort,
        _ => ConvertError::InvalidScheme,
    })?;
    if url.scheme() != "vless" {
        return Err(ConvertError::InvalidScheme);
    }

    let uuid = url.username();
    if uuid.is_empty() {
        return Err(ConvertError::MissingUuid);
    }

    let server = url
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or(ConvertError::MissingServer)?
        .to_string();

    let params: BTreeMap<String, String> = url.query_pairs().into_owned().collect();
    let security = first_value(&params, &["security"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    let tls = matches!(security.as_str(), "tls" | "reality");
    let network = normalize_network(first_value(&params, &["type", "network"]));
    let port = url
        .port()
        .or(Some(if tls { 443 } else { 80 }))
        .ok_or(ConvertError::InvalidPort)?;

    let mut proxy = Proxy {
        name: url
            .fragment()
            .filter(|fragment| !fragment.is_empty())
            .map(percent_decode)
            .unwrap_or_else(|| server.clone()),
        proxy_type: "vless".to_string(),
        server,
        port,
        uuid: uuid.to_string(),
        udp: true,
        tls,
        network,
        flow: first_value(&params, &["flow"]),
        servername: first_value(&params, &["sni", "servername", "peer"]),
        client_fingerprint: first_value(&params, &["fp", "fingerprint", "client-fingerprint"]),
        alpn: parse_alpn(first_value(&params, &["alpn"])),
        skip_cert_verify: parse_bool(first_value(&params, &["allowInsecure", "skip-cert-verify"])),
        reality_opts: None,
        packet_encoding: first_value(&params, &["packetEncoding", "packet-encoding"]),
        ws_opts: None,
        grpc_opts: None,
        httpupgrade_opts: None,
        xhttp_opts: None,
        h2_opts: None,
    };

    if security == "reality" {
        let reality_opts = RealityOptions {
            public_key: first_value(&params, &["pbk", "public-key"]),
            short_id: first_value(&params, &["sid", "short-id"]),
            spider_x: first_value(&params, &["spx", "spider-x"]),
        };
        if reality_opts.has_values() {
            proxy.reality_opts = Some(reality_opts);
        }
    }

    apply_network_options(&mut proxy, &params);

    Ok(proxy)
}

fn build_config(
    proxies: Vec<Proxy>,
    template_mode: TemplateMode,
    direct_domains: &[String],
) -> MihomoConfig {
    let proxy_names = proxies
        .iter()
        .map(|proxy| proxy.name.clone())
        .collect::<Vec<_>>();
    let include_standard = matches!(
        template_mode,
        TemplateMode::Standard | TemplateMode::FullRules
    );
    let include_full_rules = matches!(template_mode, TemplateMode::FullRules);
    let proxy_groups = build_proxy_groups(&proxy_names, &template_mode);

    MihomoConfig {
        mixed_port: 7890,
        allow_lan: false,
        bind_address: include_standard.then(|| "*".to_string()),
        mode: "rule".to_string(),
        log_level: "info".to_string(),
        dns: include_standard.then(build_dns_config),
        proxies,
        proxy_groups,
        rule_providers: include_full_rules.then(build_rule_providers),
        rules: build_rules(&template_mode, direct_domains),
    }
}

fn build_proxy_groups(proxy_names: &[String], template_mode: &TemplateMode) -> Vec<ProxyGroup> {
    let mut proxy_choices = proxy_names.to_vec();
    proxy_choices.push("AUTO".to_string());
    proxy_choices.push("DIRECT".to_string());

    let mut category_choices = vec!["PROXY".to_string()];
    category_choices.extend(proxy_names.iter().cloned());
    category_choices.push("AUTO".to_string());
    category_choices.push("DIRECT".to_string());

    match template_mode {
        TemplateMode::Minimal => {
            let mut choices = proxy_names.to_vec();
            choices.push("DIRECT".to_string());
            vec![select_group("PROXY", choices)]
        }
        TemplateMode::Standard => vec![
            select_group("PROXY", proxy_choices),
            url_test_group("AUTO", proxy_names.to_vec()),
        ],
        TemplateMode::FullRules => vec![
            select_group("PROXY", proxy_choices),
            url_test_group("AUTO", proxy_names.to_vec()),
            select_group("AI", category_choices.clone()),
            select_group("Media", category_choices.clone()),
            select_group("Google", category_choices.clone()),
            select_group("Telegram", category_choices.clone()),
            select_group("TikTok", category_choices),
            select_group("Ads", vec!["REJECT".to_string(), "DIRECT".to_string()]),
        ],
    }
}

fn select_group(name: &str, proxies: Vec<String>) -> ProxyGroup {
    ProxyGroup {
        name: name.to_string(),
        group_type: "select".to_string(),
        proxies,
        url: None,
        interval: None,
        tolerance: None,
    }
}

fn url_test_group(name: &str, proxies: Vec<String>) -> ProxyGroup {
    ProxyGroup {
        name: name.to_string(),
        group_type: "url-test".to_string(),
        proxies,
        url: Some("https://www.gstatic.com/generate_204".to_string()),
        interval: Some(300),
        tolerance: Some(50),
    }
}

fn build_dns_config() -> DnsConfig {
    DnsConfig {
        enable: true,
        ipv6: true,
        enhanced_mode: "fake-ip".to_string(),
        fake_ip_range: "198.18.0.1/16".to_string(),
        cache_algorithm: Some("arc".to_string()),
        default_nameserver: vec![
            "223.5.5.5".to_string(),
            "119.29.29.29".to_string(),
            "114.114.114.114".to_string(),
        ],
        nameserver: vec![
            "https://doh.pub/dns-query".to_string(),
            "https://dns.alidns.com/dns-query".to_string(),
        ],
        fallback: vec![
            "tls://1.1.1.1".to_string(),
            "tls://8.8.8.8".to_string(),
            "https://doh.pub/dns-query".to_string(),
        ],
        fallback_filter: DnsFallbackFilter {
            geoip: true,
            geoip_code: "CN".to_string(),
            geosite: vec!["gfw".to_string()],
            ipcidr: vec![
                "240.0.0.0/4".to_string(),
                "0.0.0.0/32".to_string(),
                "127.0.0.1/8".to_string(),
            ],
        },
        respect_rules: false,
        proxy_server_nameserver: vec![
            "https://doh.pub/dns-query".to_string(),
            "https://dns.alidns.com/dns-query".to_string(),
        ],
        fake_ip_filter: vec![
            "*.lan".to_string(),
            "*.localdomain".to_string(),
            "*.localhost".to_string(),
            "*.local".to_string(),
            "time.*.com".to_string(),
            "ntp.*.com".to_string(),
            "+.pool.ntp.org".to_string(),
            "+.msftconnecttest.com".to_string(),
            "+.msftncsi.com".to_string(),
            "localhost.ptlogin2.qq.com".to_string(),
            "localhost.sec.qq.com".to_string(),
            "mtalk.google.com".to_string(),
        ],
    }
}

fn build_rule_providers() -> BTreeMap<String, RuleProvider> {
    let mut providers = BTreeMap::new();
    providers.insert(
        "reject".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/category-ads-all.yaml",
            "./ruleset/reject.yaml",
        ),
    );
    providers.insert(
        "openai".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/openai.yaml",
            "./ruleset/openai.yaml",
        ),
    );
    providers.insert(
        "youtube".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/youtube.yaml",
            "./ruleset/youtube.yaml",
        ),
    );
    providers.insert(
        "netflix".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/netflix.yaml",
            "./ruleset/netflix.yaml",
        ),
    );
    providers.insert(
        "telegram".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/telegram.yaml",
            "./ruleset/telegram.yaml",
        ),
    );
    providers.insert(
        "tiktok".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/tiktok.yaml",
            "./ruleset/tiktok.yaml",
        ),
    );
    providers.insert(
        "google_domain".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/google.yaml",
            "./ruleset/google-domain.yaml",
        ),
    );
    providers.insert(
        "google_ip".to_string(),
        http_rule_provider(
            "ipcidr",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geoip/google.yaml",
            "./ruleset/google-ip.yaml",
        ),
    );
    providers.insert(
        "geolocation_not_cn".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/geolocation-!cn.yaml",
            "./ruleset/geolocation-not-cn.yaml",
        ),
    );
    providers.insert(
        "cn_domain".to_string(),
        http_rule_provider(
            "domain",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geosite/cn.yaml",
            "./ruleset/cn-domain.yaml",
        ),
    );
    providers.insert(
        "cn_ip".to_string(),
        http_rule_provider(
            "ipcidr",
            "https://cdn.jsdmirror.com/gh/MetaCubeX/meta-rules-dat@meta/geo/geoip/cn.yaml",
            "./ruleset/cn-ip.yaml",
        ),
    );
    providers
}

fn http_rule_provider(behavior: &str, url: &str, path: &str) -> RuleProvider {
    RuleProvider {
        provider_type: "http".to_string(),
        behavior: behavior.to_string(),
        format: Some("yaml".to_string()),
        interval: Some(86400),
        url: Some(url.to_string()),
        path: Some(path.to_string()),
        payload: None,
    }
}

fn build_rules(template_mode: &TemplateMode, direct_domains: &[String]) -> Vec<String> {
    match template_mode {
        TemplateMode::Minimal => with_local_direct_rules(vec!["MATCH,PROXY"], direct_domains),
        TemplateMode::Standard => {
            with_local_direct_rules(vec!["GEOIP,CN,DIRECT", "MATCH,PROXY"], direct_domains)
        }
        TemplateMode::FullRules => with_local_direct_rules(
            vec![
                "RULE-SET,reject,Ads",
                "RULE-SET,openai,AI",
                "RULE-SET,youtube,Media",
                "RULE-SET,netflix,Media",
                "RULE-SET,telegram,Telegram",
                "RULE-SET,tiktok,TikTok",
                "RULE-SET,google_domain,Google",
                "RULE-SET,google_ip,Google,no-resolve",
                "RULE-SET,geolocation_not_cn,PROXY",
                "RULE-SET,cn_domain,DIRECT",
                "RULE-SET,cn_ip,DIRECT,no-resolve",
                "GEOIP,CN,DIRECT",
                "MATCH,PROXY",
            ],
            direct_domains,
        ),
    }
}

fn with_local_direct_rules(rules: Vec<&'static str>, direct_domains: &[String]) -> Vec<String> {
    let mut merged = vec![
        "DOMAIN,localhost,DIRECT",
        "DOMAIN-SUFFIX,localhost,DIRECT",
        "DOMAIN-SUFFIX,local,DIRECT",
        "DOMAIN-SUFFIX,lan,DIRECT",
        "DST-PORT,22,DIRECT",
        "IP-CIDR,127.0.0.0/8,DIRECT,no-resolve",
        "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
        "IP-CIDR,100.64.0.0/10,DIRECT,no-resolve",
        "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
        "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
        "IP-CIDR,169.254.0.0/16,DIRECT,no-resolve",
        "IP-CIDR6,::1/128,DIRECT,no-resolve",
        "IP-CIDR6,fc00::/7,DIRECT,no-resolve",
        "IP-CIDR6,fe80::/10,DIRECT,no-resolve",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>();

    let mut seen = merged.iter().cloned().collect::<BTreeSet<_>>();
    for rule in build_custom_direct_domain_rules(direct_domains) {
        if seen.insert(rule.clone()) {
            merged.push(rule);
        }
    }

    merged.extend(rules.into_iter().map(ToOwned::to_owned));
    merged
}

fn build_custom_direct_domain_rules(direct_domains: &[String]) -> Vec<String> {
    direct_domains
        .iter()
        .filter_map(|domain| normalize_direct_domain(domain))
        .map(|domain| format!("DOMAIN-SUFFIX,{domain},DIRECT"))
        .collect()
}

fn normalize_direct_domain(value: &str) -> Option<String> {
    let without_comment = value.split('#').next().unwrap_or_default().trim();
    if without_comment.is_empty() {
        return None;
    }

    let host = if without_comment.contains("://") {
        Url::parse(without_comment)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .unwrap_or_default()
    } else {
        without_comment
            .split('/')
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_string()
    };

    let normalized = host
        .trim()
        .trim_start_matches("*.")
        .trim_start_matches("+.")
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();

    if normalized.is_empty()
        || normalized.contains(',')
        || normalized.chars().any(char::is_whitespace)
    {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_network(network: Option<String>) -> String {
    let network = network.map(|value| value.to_ascii_lowercase());
    match network.as_deref() {
        Some("websocket") => "ws".to_string(),
        Some("http") => "h2".to_string(),
        Some(value) if !value.is_empty() => value.to_string(),
        _ => "tcp".to_string(),
    }
}

fn apply_network_options(proxy: &mut Proxy, params: &BTreeMap<String, String>) {
    match proxy.network.as_str() {
        "ws" => {
            let headers = first_value(params, &["host"]).map(|host| {
                let mut headers = BTreeMap::new();
                headers.insert("Host".to_string(), host);
                headers
            });

            let ws_opts = WebSocketOptions {
                path: first_value(params, &["path"]),
                headers,
            };
            if ws_opts.has_values() {
                proxy.ws_opts = Some(ws_opts);
            }
        }
        "grpc" => {
            let grpc_opts = GrpcOptions {
                grpc_service_name: first_value(params, &["serviceName", "service-name", "path"]),
                grpc_mode: first_value(params, &["mode"]),
                grpc_authority: first_value(params, &["authority", "host"]),
            };
            if grpc_opts.has_values() {
                proxy.grpc_opts = Some(grpc_opts);
            }
        }
        "httpupgrade" => {
            let httpupgrade_opts = HttpUpgradeOptions {
                path: first_value(params, &["path"]),
                host: first_value(params, &["host"]).map(|host| vec![host]),
            };
            if httpupgrade_opts.has_values() {
                proxy.httpupgrade_opts = Some(httpupgrade_opts);
            }
        }
        "xhttp" => {
            let xhttp_opts = XHttpOptions {
                path: first_value(params, &["path"]),
                host: first_value(params, &["host"]),
                mode: first_value(params, &["mode"]),
            };
            if xhttp_opts.has_values() {
                proxy.xhttp_opts = Some(xhttp_opts);
            }
        }
        "h2" => {
            let h2_opts = H2Options {
                path: first_value(params, &["path"]),
                host: first_value(params, &["host"]).map(|host| vec![host]),
            };
            if h2_opts.has_values() {
                proxy.h2_opts = Some(h2_opts);
            }
        }
        _ => {}
    }
}

fn first_value(params: &BTreeMap<String, String>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        params
            .get(*name)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn parse_bool(value: Option<String>) -> Option<bool> {
    value.map(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn parse_alpn(value: Option<String>) -> Option<Vec<String>> {
    value
        .map(|value| {
            value
                .split(',')
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
}

fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .into_owned()
}

fn normalize_custom_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ensure_unique_proxy_names(proxies: &mut [Proxy]) {
    let mut used = BTreeSet::new();
    for proxy in proxies {
        if used.insert(proxy.name.clone()) {
            continue;
        }

        let base_name = proxy.name.clone();
        for index in 2.. {
            let candidate = format!("{base_name}-{index}");
            if used.insert(candidate.clone()) {
                proxy.name = candidate;
                break;
            }
        }
    }
}

fn format_node_addresses_comment(node_addresses: &[String]) -> String {
    if node_addresses.len() <= 1 {
        return format!(
            "# 节点地址: {}\n",
            node_addresses.first().cloned().unwrap_or_default()
        );
    }

    format!("# 节点地址: {}\n", node_addresses.join(", "))
}

impl RealityOptions {
    fn has_values(&self) -> bool {
        self.public_key.is_some() || self.short_id.is_some() || self.spider_x.is_some()
    }
}

impl WebSocketOptions {
    fn has_values(&self) -> bool {
        self.path.is_some() || self.headers.is_some()
    }
}

impl GrpcOptions {
    fn has_values(&self) -> bool {
        self.grpc_service_name.is_some()
            || self.grpc_mode.is_some()
            || self.grpc_authority.is_some()
    }
}

impl HttpUpgradeOptions {
    fn has_values(&self) -> bool {
        self.path.is_some() || self.host.is_some()
    }
}

impl XHttpOptions {
    fn has_values(&self) -> bool {
        self.path.is_some() || self.host.is_some() || self.mode.is_some()
    }
}

impl H2Options {
    fn has_values(&self) -> bool {
        self.path.is_some() || self.host.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    #[test]
    fn converts_reality_tcp_to_full_config() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123&fp=chrome&sni=www.microsoft.com&sid=abcd&flow=xtls-rprx-vision#test-reality";

        let yaml = convert_vless_to_yaml(input, ConvertOptions::default()).unwrap();

        assert!(yaml.starts_with("# 节点地址: example.com:443\n"));
        assert!(yaml.contains("mixed-port: 7890"));
        assert!(yaml.contains("type: vless"));
        assert!(yaml.contains("servername: www.microsoft.com"));
        assert!(yaml.contains("client-fingerprint: chrome"));
        assert!(yaml.contains("public-key: abc123"));
        assert!(yaml.contains("short-id: abcd"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn converts_standard_template_with_dns_and_auto_group() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123&fp=chrome&sni=www.microsoft.com&sid=abcd&flow=xtls-rprx-vision#test-reality";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::FullConfig,
                template_mode: TemplateMode::Standard,
                proxy_name: None,
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.contains("dns:"));
        assert!(yaml.contains("enhanced-mode: fake-ip"));
        assert!(yaml.contains("name: AUTO"));
        assert!(yaml.contains("type: url-test"));
        assert!(yaml.contains("DOMAIN,localhost,DIRECT"));
        assert!(yaml.contains("DST-PORT,22,DIRECT"));
        assert!(yaml.contains("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
        assert!(yaml.contains("IP-CIDR6,::1/128,DIRECT,no-resolve"));
        assert!(yaml.find("DST-PORT,22,DIRECT") < yaml.find("MATCH,PROXY"));
        assert!(yaml.find("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve") < yaml.find("MATCH,PROXY"));
        assert!(yaml.find("IP-CIDR6,::1/128,DIRECT,no-resolve") < yaml.find("MATCH,PROXY"));
        assert!(yaml.contains("GEOIP,CN,DIRECT"));
        assert!(!yaml.contains("rule-providers:"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn keeps_ssh_port_direct_rule_before_ip_family_rules_and_fallback() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123#test-reality";

        let yaml = convert_vless_to_yaml(input, ConvertOptions::default()).unwrap();

        assert!(yaml.contains("DST-PORT,22,DIRECT"));
        assert!(yaml.contains("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve"));
        assert!(yaml.contains("IP-CIDR6,::1/128,DIRECT,no-resolve"));
        assert!(
            yaml.find("DST-PORT,22,DIRECT") < yaml.find("IP-CIDR,127.0.0.0/8,DIRECT,no-resolve")
        );
        assert!(yaml.find("DST-PORT,22,DIRECT") < yaml.find("IP-CIDR6,::1/128,DIRECT,no-resolve"));
        assert!(yaml.find("DST-PORT,22,DIRECT") < yaml.find("MATCH,PROXY"));
    }

    #[test]
    fn converts_full_rules_template_with_rule_providers() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123&fp=chrome&sni=www.microsoft.com&sid=abcd&flow=xtls-rprx-vision#test-reality";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::FullConfig,
                template_mode: TemplateMode::FullRules,
                proxy_name: None,
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.contains("rule-providers:"));
        assert!(yaml.contains("openai:"));
        assert!(yaml.contains("youtube:"));
        assert!(yaml.contains("geolocation_not_cn:"));
        assert!(yaml.contains("RULE-SET,openai,AI"));
        assert!(yaml.contains("RULE-SET,geolocation_not_cn,PROXY"));
        assert!(yaml.contains("RULE-SET,cn_ip,DIRECT,no-resolve"));
        assert!(yaml.contains("GEOIP,CN,DIRECT"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn converts_multiple_vless_links_into_one_full_config() {
        let input = [
            "vless://11111111-1111-1111-1111-111111111111@example-a.com:443?type=tcp&security=reality&pbk=abc123#node-a",
            "vless://22222222-2222-2222-2222-222222222222@example-b.com:8443?type=tcp&security=reality&pbk=def456#node-b",
        ]
        .join("\n");

        let yaml = convert_vless_to_yaml(
            &input,
            ConvertOptions {
                output_mode: OutputMode::FullConfig,
                template_mode: TemplateMode::FullRules,
                proxy_name: Some("ignored-for-multiple".to_string()),
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.starts_with("# 节点地址: example-a.com:443, example-b.com:8443\n"));
        assert!(yaml.contains("server: example-a.com"));
        assert!(yaml.contains("server: example-b.com"));
        assert!(yaml.contains("- node-a"));
        assert!(yaml.contains("- node-b"));
        assert!(yaml.contains("name: AUTO"));
        assert!(yaml.contains("RULE-SET,openai,AI"));
        assert!(!yaml.contains("ignored-for-multiple"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn keeps_duplicate_proxy_names_unique() {
        let input = [
            "vless://11111111-1111-1111-1111-111111111111@example-a.com:443?type=tcp&security=reality&pbk=abc123#same",
            "vless://22222222-2222-2222-2222-222222222222@example-b.com:443?type=tcp&security=reality&pbk=def456#same",
        ]
        .join("\n");

        let yaml = convert_vless_to_yaml(&input, ConvertOptions::default()).unwrap();

        assert!(yaml.contains("name: same"));
        assert!(yaml.contains("name: same-2"));
        assert!(yaml.contains("same-2"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn converts_ws_to_proxy_only() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=ws&security=tls&path=%2Fws&host=cdn.example.com&sni=cdn.example.com#test-ws";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::ProxyOnly,
                template_mode: TemplateMode::Minimal,
                proxy_name: None,
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.contains("network: ws"));
        assert!(yaml.contains("path: /ws"));
        assert!(yaml.contains("Host: cdn.example.com"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn converts_grpc_to_proxy_only() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=grpc&security=tls&serviceName=my-grpc&sni=example.com#test-grpc";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::ProxyOnly,
                template_mode: TemplateMode::Minimal,
                proxy_name: None,
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.contains("network: grpc"));
        assert!(yaml.contains("grpc-service-name: my-grpc"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn rejects_non_vless_links() {
        let error =
            convert_vless_to_yaml("https://example.com", ConvertOptions::default()).unwrap_err();

        assert_eq!(error, ConvertError::InvalidScheme);
    }

    #[test]
    fn overrides_proxy_name_when_custom_name_is_present() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123#original-name";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::FullConfig,
                template_mode: TemplateMode::Standard,
                proxy_name: Some("my-node".to_string()),
                direct_domains: Vec::new(),
            },
        )
        .unwrap();

        assert!(yaml.contains("name: my-node"));
        assert!(yaml.contains("- my-node"));
        assert!(!yaml.contains("original-name"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }

    #[test]
    fn adds_custom_direct_domains_before_proxy_fallback() {
        let input = "vless://11111111-1111-1111-1111-111111111111@example.com:443?type=tcp&security=reality&pbk=abc123#test-reality";

        let yaml = convert_vless_to_yaml(
            input,
            ConvertOptions {
                output_mode: OutputMode::FullConfig,
                template_mode: TemplateMode::FullRules,
                proxy_name: None,
                direct_domains: vec![
                    "github.com".to_string(),
                    "https://example.org/docs".to_string(),
                    "*.internal.test".to_string(),
                ],
            },
        )
        .unwrap();

        assert!(yaml.contains("DOMAIN-SUFFIX,github.com,DIRECT"));
        assert!(yaml.contains("DOMAIN-SUFFIX,example.org,DIRECT"));
        assert!(yaml.contains("DOMAIN-SUFFIX,internal.test,DIRECT"));
        assert!(
            yaml.find("DOMAIN-SUFFIX,github.com,DIRECT")
                < yaml.find("RULE-SET,geolocation_not_cn,PROXY")
        );
        assert!(yaml.find("DOMAIN-SUFFIX,github.com,DIRECT") < yaml.find("MATCH,PROXY"));
        serde_yaml::from_str::<Value>(&yaml).unwrap();
    }
}

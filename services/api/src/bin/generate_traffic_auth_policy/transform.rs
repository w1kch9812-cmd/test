//! Pure string/number transforms shared across generator phases.
//!
//! These mirror the helper functions in the retired
//! `scripts/ci/traffic-auth-policy-generator/shared.ps1` and
//! `rust-edge-shared.ps1` so the generated output is byte-for-byte identical.

/// Groups a non-negative integer into underscore-separated thousands.
///
/// Matches `Format-NumberLiteral` (e.g. `262144` -> `262_144`).
pub fn format_number_literal(value: i64) -> String {
    let mut digits = value.to_string();
    if digits.len() <= 3 {
        return digits;
    }
    let mut groups: Vec<String> = Vec::new();
    while digits.len() > 3 {
        let split = digits.len() - 3;
        groups.insert(0, digits[split..].to_string());
        digits = digits[..split].to_string();
    }
    groups.insert(0, digits);
    groups.join("_")
}

/// Escapes a string for a TypeScript/Rust double-quoted literal.
///
/// Matches `Convert-StringToTs` / `Convert-PathSourceToTs` (`\` then `"`).
pub fn escape_double_quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Renders a string array as a TypeScript array literal of quoted strings.
///
/// Matches `Convert-StringArrayToTs`.
pub fn string_array_to_ts(values: &[String]) -> String {
    let quoted: Vec<String> = values
        .iter()
        .map(|value| format!("\"{}\"", escape_double_quoted(value)))
        .collect();
    format!("[{}]", quoted.join(", "))
}

/// Derives a camelCase operation name from a dotted policy id.
///
/// Matches `Convert-PolicyIdToOperationName`.
pub fn policy_id_to_operation_name(id: &str) -> Result<String, String> {
    let leaf = id.rfind('.').map_or(id, |index| &id[index + 1..]);
    let parts: Vec<&str> = leaf.split('_').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Err(format!(
            "Cannot derive operation name from policy id '{id}'"
        ));
    }
    let mut name = parts[0].to_string();
    for part in &parts[1..] {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.push_str(&first.to_uppercase().to_string());
            name.push_str(chars.as_str());
        }
    }
    Ok(name)
}

/// Extracts `:param` names from an API proxy target path.
///
/// Matches `Get-ApiProxyPathParameterNames`.
pub fn api_proxy_path_parameter_names(target_path: &str) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for segment in target_path.split('/') {
        if let Some(name) = segment.strip_prefix(':') {
            if !is_identifier(name) {
                return Err(format!(
                    "Unsupported API proxy template parameter '{name}' in '{target_path}'"
                ));
            }
            names.push(name.to_string());
        }
    }
    Ok(names)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Escapes a literal segment for embedding in a TS template string.
///
/// Matches `Convert-StringToTsTemplateSegment` (`\`, backtick, `$`).
fn escape_template_segment(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

/// Renders an API proxy target path as a TS path expression.
///
/// Returns a backtick template when the path has parameters, otherwise a
/// double-quoted string literal. Matches
/// `Convert-ApiProxyTargetPathToTsExpression`.
pub fn api_proxy_target_path_to_ts_expression(target_path: &str) -> Result<String, String> {
    let mut parts: Vec<String> = Vec::new();
    let mut has_parameter = false;
    for segment in target_path.split('/') {
        if segment.is_empty() {
            continue;
        }
        if let Some(name) = segment.strip_prefix(':') {
            has_parameter = true;
            parts.push(format!("${{encodePathParam(params.{name})}}"));
        } else {
            parts.push(escape_template_segment(segment));
        }
    }
    if parts.is_empty() {
        return Err("API proxy target_path cannot be empty".to_string());
    }
    let path = parts.join("/");
    if has_parameter {
        Ok(format!("`{path}`"))
    } else {
        Ok(format!("\"{}\"", escape_double_quoted(&path)))
    }
}

/// Renders the `params` type for an API proxy operation.
///
/// Matches `Format-ApiProxyParamsType`.
pub fn api_proxy_params_type(names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let fields: Vec<String> = names
        .iter()
        .map(|name| format!("readonly {name}: string"))
        .collect();
    format!("{{ {} }}", fields.join("; "))
}

/// Maps an HTTP method to the ky request method name.
///
/// Matches `Get-RequestMethodName`.
pub fn request_method_name(method: &str) -> Result<&'static str, String> {
    match method {
        "GET" => Ok("get"),
        "POST" => Ok("post"),
        "PUT" => Ok("put"),
        "PATCH" => Ok("patch"),
        "DELETE" => Ok("delete"),
        other => Err(format!("Unsupported API proxy client method '{other}'")),
    }
}

/// Converts a `{param}` backend path into a `:param` Rust router pattern.
///
/// Matches `Convert-BackendPolicyPathToRustPattern`.
pub fn backend_policy_path_to_rust_pattern(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars();
    while let Some(character) = chars.next() {
        if character == '{' {
            let mut name = String::new();
            for inner in chars.by_ref() {
                if inner == '}' {
                    break;
                }
                name.push(inner);
            }
            result.push(':');
            result.push_str(&name);
        } else {
            result.push(character);
        }
    }
    result
}

/// Maps a rate key strategy to a Rust `BackendRateKeyStrategy` variant.
///
/// Matches `Convert-KeyStrategyToRust`.
pub fn key_strategy_to_rust(strategy: &str) -> Result<&'static str, String> {
    match strategy {
        "client_ip" => Ok("BackendRateKeyStrategy::ClientIp"),
        "session_sub" => Ok("BackendRateKeyStrategy::SessionSub"),
        other => Err(format!("Unsupported route rate key_strategy '{other}'")),
    }
}

/// Maps a role name to a Rust `UserRole` variant.
///
/// Matches `Convert-RoleToRust`.
pub fn role_to_rust(role: &str) -> Result<&'static str, String> {
    match role {
        "Admin" => Ok("UserRole::Admin"),
        "Broker" => Ok("UserRole::Broker"),
        "Buyer" => Ok("UserRole::Buyer"),
        "Developer" => Ok("UserRole::Developer"),
        "Enterprise" => Ok("UserRole::Enterprise"),
        "Operator" => Ok("UserRole::Operator"),
        "Seller" => Ok("UserRole::Seller"),
        other => Err(format!("Unsupported backend required role '{other}'")),
    }
}

/// Maps a path kind to an AWS `WAFv2` path match operator.
///
/// Matches `Convert-PathKindToAwsWafPathMatch`.
pub fn path_kind_to_aws_waf_path_match(kind: &str) -> Result<&'static str, String> {
    match kind {
        "exact" => Ok("EXACT"),
        "prefix" => Ok("STARTS_WITH"),
        other => Err(format!("Unsupported AWS WAFv2 path kind '{other}'")),
    }
}

/// Computes the 5-minute rate limit from a per-window rate.
///
/// Matches `Convert-RateToFiveMinuteLimit` (`ceil(limit * 300 / window)`).
/// Uses integer ceiling arithmetic so the result is exact and free of any
/// floating-point rounding.
pub fn rate_to_five_minute_limit(
    limit: i64,
    window_seconds: i64,
    key_prefix: &str,
) -> Result<i64, String> {
    if window_seconds <= 0 {
        return Err(format!(
            "Rate window_seconds must be positive for {key_prefix}"
        ));
    }
    let numerator = limit
        .checked_mul(300)
        .ok_or_else(|| "rate limit overflow".to_string())?;
    Ok((numerator + window_seconds - 1) / window_seconds)
}

/// Resolves an auth `path_source` token to its concrete request path.
///
/// Matches `Resolve-AuthPathSource`.
pub fn resolve_auth_path_source(path_source: &str) -> Result<&'static str, String> {
    match path_source {
        "API.auth.login" => Ok("/api/auth/login"),
        "API.auth.callback" => Ok("/api/auth/callback"),
        "API.auth.refresh" => Ok("/api/auth/refresh"),
        "API.auth.logout" => Ok("/api/auth/logout"),
        other => Err(format!("Unsupported auth path source '{other}'")),
    }
}

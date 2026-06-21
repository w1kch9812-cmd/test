//! Pure string/number transforms shared across generator phases.
//!
//! These mirror the helper functions in the retired generator's shared and
//! rust/edge helper modules so the generated output is byte-for-byte identical.

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
    // Integer ceiling: (numerator + window - 1) / window. Guard the addition too
    // so an extreme limit cannot overflow i64 between the multiply and divide.
    let rounded = numerator
        .checked_add(window_seconds - 1)
        .ok_or_else(|| "rate limit overflow".to_string())?;
    Ok(rounded / window_seconds)
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    // ------------------------------------------------------------------
    // rate_to_five_minute_limit — security WAF rate translation.
    // result = ceil(limit * 300 / window), with overflow + window guards.
    // ------------------------------------------------------------------

    #[test]
    fn rate_to_five_minute_limit_exact_division_has_no_ceiling() {
        // 60 req / 60s == 1 req/s == 300 req / 5min, divides exactly.
        assert_eq!(rate_to_five_minute_limit(60, 60, "k").unwrap(), 300);
    }

    #[test]
    fn rate_to_five_minute_limit_rounds_up_on_remainder() {
        // 10 * 300 = 3000; 3000 / 7 = 428.57..., ceiling -> 429.
        assert_eq!(rate_to_five_minute_limit(10, 7, "k").unwrap(), 429);
        // 1 * 300 = 300; 300 / 299 = 1.003..., ceiling -> 2.
        assert_eq!(rate_to_five_minute_limit(1, 299, "k").unwrap(), 2);
        // 1 * 300 = 300; 300 / 300 = 1 exactly, no rounding.
        assert_eq!(rate_to_five_minute_limit(1, 300, "k").unwrap(), 1);
        // 1 * 300 = 300; 300 / 301 = 0.99..., ceiling -> 1.
        assert_eq!(rate_to_five_minute_limit(1, 301, "k").unwrap(), 1);
    }

    #[test]
    fn rate_to_five_minute_limit_zero_limit_is_zero() {
        // A zero base rate must not round up to one.
        assert_eq!(rate_to_five_minute_limit(0, 60, "k").unwrap(), 0);
    }

    #[test]
    fn rate_to_five_minute_limit_window_window_one_second() {
        // 5 req / 1s -> 5 * 300 = 1500 over 5 minutes.
        assert_eq!(rate_to_five_minute_limit(5, 1, "k").unwrap(), 1500);
    }

    #[test]
    fn rate_to_five_minute_limit_rejects_zero_window() {
        let error = rate_to_five_minute_limit(10, 0, "auth-login").unwrap_err();
        assert!(error.contains("window_seconds must be positive"));
        assert!(error.contains("auth-login"));
    }

    #[test]
    fn rate_to_five_minute_limit_rejects_negative_window() {
        let error = rate_to_five_minute_limit(10, -5, "kp").unwrap_err();
        assert!(error.contains("window_seconds must be positive"));
        assert!(error.contains("kp"));
    }

    #[test]
    fn rate_to_five_minute_limit_detects_checked_mul_overflow() {
        // limit * 300 overflows i64 -> checked_mul returns None -> Err.
        let error = rate_to_five_minute_limit(i64::MAX, 60, "kp").unwrap_err();
        assert_eq!(error, "rate limit overflow");
    }

    #[test]
    fn rate_to_five_minute_limit_realistic_large_limit() {
        // A large-but-realistic per-minute rate translates without overflow.
        // 10_000 / 60s -> ceil(10_000 * 300 / 60) = ceil(3_000_000 / 60) = 50_000.
        assert_eq!(rate_to_five_minute_limit(10_000, 60, "kp").unwrap(), 50_000);
    }

    #[test]
    fn rate_to_five_minute_limit_detects_ceiling_add_overflow() {
        // limit * 300 just fits i64, but the ceiling (+ window - 1) addition would
        // overflow; the checked_add guard turns it into an error, not a panic.
        let max_limit = i64::MAX / 300;
        let error = rate_to_five_minute_limit(max_limit, 300, "kp").unwrap_err();
        assert_eq!(error, "rate limit overflow");
    }

    // ------------------------------------------------------------------
    // format_number_literal — thousands grouping for emitted code.
    // ------------------------------------------------------------------

    #[test]
    fn format_number_literal_short_values_unchanged() {
        assert_eq!(format_number_literal(0), "0");
        assert_eq!(format_number_literal(7), "7");
        assert_eq!(format_number_literal(42), "42");
        assert_eq!(format_number_literal(999), "999");
    }

    #[test]
    fn format_number_literal_groups_thousands() {
        assert_eq!(format_number_literal(1000), "1_000");
        assert_eq!(format_number_literal(262_144), "262_144");
        assert_eq!(format_number_literal(1_000_000), "1_000_000");
        assert_eq!(format_number_literal(12_345_678), "12_345_678");
    }

    #[test]
    fn format_number_literal_boundary_four_digits() {
        assert_eq!(format_number_literal(1234), "1_234");
        assert_eq!(format_number_literal(9999), "9_999");
    }

    // ------------------------------------------------------------------
    // policy_id_to_operation_name — camelCase op name from dotted id.
    // ------------------------------------------------------------------

    #[test]
    fn policy_id_to_operation_name_takes_leaf_after_last_dot() {
        assert_eq!(
            policy_id_to_operation_name("api_proxy.listings.create_listing").unwrap(),
            "createListing"
        );
    }

    #[test]
    fn policy_id_to_operation_name_single_word_leaf() {
        assert_eq!(
            policy_id_to_operation_name("group.search").unwrap(),
            "search"
        );
    }

    #[test]
    fn policy_id_to_operation_name_no_dot_uses_whole_id() {
        assert_eq!(
            policy_id_to_operation_name("get_user_profile").unwrap(),
            "getUserProfile"
        );
    }

    #[test]
    fn policy_id_to_operation_name_collapses_repeated_underscores() {
        // Empty underscore-split parts are filtered out.
        assert_eq!(policy_id_to_operation_name("a__b___c").unwrap(), "aBC");
    }

    #[test]
    fn policy_id_to_operation_name_errors_on_empty_leaf() {
        // Trailing dot -> empty leaf -> no parts.
        let error = policy_id_to_operation_name("group.").unwrap_err();
        assert!(error.contains("Cannot derive operation name"));
    }

    #[test]
    fn policy_id_to_operation_name_errors_on_only_underscores() {
        let error = policy_id_to_operation_name("___").unwrap_err();
        assert!(error.contains("Cannot derive operation name"));
    }

    // ------------------------------------------------------------------
    // api_proxy_target_path_to_ts_expression — quoted literal vs template.
    // ------------------------------------------------------------------

    #[test]
    fn api_proxy_target_path_static_returns_quoted_literal() {
        assert_eq!(
            api_proxy_target_path_to_ts_expression("/listings/search").unwrap(),
            "\"listings/search\""
        );
    }

    #[test]
    fn api_proxy_target_path_with_param_returns_backtick_template() {
        assert_eq!(
            api_proxy_target_path_to_ts_expression("/listings/:id").unwrap(),
            "`listings/${encodePathParam(params.id)}`"
        );
    }

    #[test]
    fn api_proxy_target_path_with_multiple_params() {
        assert_eq!(
            api_proxy_target_path_to_ts_expression("/users/:userId/listings/:listingId").unwrap(),
            "`users/${encodePathParam(params.userId)}/listings/${encodePathParam(params.listingId)}`"
        );
    }

    #[test]
    fn api_proxy_target_path_strips_empty_segments() {
        // Leading slash and doubled slashes collapse via the empty-segment skip.
        assert_eq!(
            api_proxy_target_path_to_ts_expression("//a//b/").unwrap(),
            "\"a/b\""
        );
    }

    #[test]
    fn api_proxy_target_path_empty_is_error() {
        let error = api_proxy_target_path_to_ts_expression("/").unwrap_err();
        assert!(error.contains("cannot be empty"));
    }

    // ------------------------------------------------------------------
    // backend_policy_path_to_rust_pattern — {param} -> :param routers.
    // ------------------------------------------------------------------

    #[test]
    fn backend_policy_path_static_unchanged() {
        assert_eq!(
            backend_policy_path_to_rust_pattern("/listings/search"),
            "/listings/search"
        );
    }

    #[test]
    fn backend_policy_path_single_param() {
        assert_eq!(
            backend_policy_path_to_rust_pattern("/listings/{id}"),
            "/listings/:id"
        );
    }

    #[test]
    fn backend_policy_path_multiple_params() {
        assert_eq!(
            backend_policy_path_to_rust_pattern("/users/{userId}/listings/{listingId}"),
            "/users/:userId/listings/:listingId"
        );
    }

    #[test]
    fn backend_policy_path_param_with_underscore() {
        assert_eq!(
            backend_policy_path_to_rust_pattern("/a/{snake_case_name}/b"),
            "/a/:snake_case_name/b"
        );
    }

    #[test]
    fn backend_policy_path_unterminated_brace_consumes_rest() {
        // No closing brace: the loop drains the iterator into the param name.
        assert_eq!(backend_policy_path_to_rust_pattern("/a/{id"), "/a/:id");
    }
}

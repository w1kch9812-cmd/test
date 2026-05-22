#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;

// Version

#[test]
fn version_valid_inputs() {
    for s in ["v3", "v0", "v_2026_05", "v-dryrun", "v_2026-05", "v3a"] {
        Version::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
    }
}

#[test]
fn version_rejects_invalid() {
    for s in ["", "V3", "3", "v", "v3!", "vA", "v 3", " v3", "v3 "] {
        assert!(
            Version::new(s).is_err(),
            "must reject {s:?} — got Ok unexpectedly"
        );
    }
}

#[test]
fn version_too_long() {
    let s = format!("v{}", "a".repeat(64));
    assert!(matches!(Version::new(s), Err(TypeError::VersionTooLong(_))));
}

#[test]
fn version_display_and_asref() {
    let v = Version::new("v3").unwrap();
    assert_eq!(v.to_string(), "v3");
    assert_eq!(v.as_ref(), "v3");
    assert_eq!(v.as_str(), "v3");
}

#[test]
fn version_serde_round_trip() {
    let v = Version::new("v_2026_05").unwrap();
    let json = serde_json::to_string(&v).unwrap();
    assert_eq!(json, "\"v_2026_05\"");
    let parsed: Version = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, v);
}

#[test]
fn version_deserialize_rejects_invalid() {
    let err = serde_json::from_str::<Version>("\"V3\"").unwrap_err();
    assert!(err.to_string().contains("v[a-z0-9_-]+"));
}

// Srs

#[test]
fn srs_valid_inputs() {
    for s in ["EPSG:4326", "EPSG:5179", "EPSG:5186", "EPSG:3857"] {
        Srs::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
    }
}

#[test]
fn srs_rejects_invalid() {
    for s in [
        "",
        "epsg:4326",
        "EPSG:",
        "EPSG:abc",
        "EPSG: 4326",
        "EPSG4326",
        "WGS84",
    ] {
        assert!(Srs::new(s).is_err(), "must reject {s:?}");
    }
}

#[test]
fn srs_epsg_code() {
    assert_eq!(Srs::new("EPSG:4326").unwrap().epsg_code(), Some(4326));
    assert_eq!(Srs::new("EPSG:5186").unwrap().epsg_code(), Some(5186));
}

#[test]
fn srs_epsg_code_overflow_returns_none() {
    // is_valid_srs 가 임의 길이 digits 통과하지만 epsg_code 는 u32 한계 → None.
    let huge = format!("EPSG:{}", "9".repeat(20));
    assert_eq!(Srs::new(huge).unwrap().epsg_code(), None);
}

#[test]
fn srs_serde() {
    let s = Srs::new("EPSG:4326").unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"EPSG:4326\"");
    let parsed: Srs = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, s);
}

// R2PublicBase

#[test]
fn r2_public_base_valid() {
    for s in [
        "https://r2.gongzzang.dev",
        "https://r2.gongzzang.dev/",
        "https://r2.example.com/path",
        "http://localhost:9000",
        "http://localhost:9000/bucket",
    ] {
        R2PublicBase::new(s).unwrap_or_else(|_| panic!("must accept {s}"));
    }
}

#[test]
fn r2_public_base_rejects_invalid_scheme() {
    for s in [
        "",
        "ftp://r2.example.com",
        "r2.example.com",
        "//r2.example.com",
    ] {
        assert!(R2PublicBase::new(s).is_err(), "must reject {s:?}");
    }
}

#[test]
fn r2_public_base_rejects_missing_host() {
    for s in ["https://", "https:///path", "http://?query"] {
        assert!(R2PublicBase::new(s).is_err(), "must reject {s:?}");
    }
}

#[test]
fn r2_public_base_serde() {
    let b = R2PublicBase::new("https://r2.example.com").unwrap();
    let json = serde_json::to_string(&b).unwrap();
    assert_eq!(json, "\"https://r2.example.com\"");
    let parsed: R2PublicBase = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, b);
}

#[test]
fn from_str_works() {
    assert_eq!(
        "v3".parse::<Version>().unwrap(),
        Version::new("v3").unwrap()
    );
    assert_eq!(
        "EPSG:4326".parse::<Srs>().unwrap(),
        Srs::new("EPSG:4326").unwrap()
    );
    assert_eq!(
        "https://r2.example.com".parse::<R2PublicBase>().unwrap(),
        R2PublicBase::new("https://r2.example.com").unwrap()
    );
}

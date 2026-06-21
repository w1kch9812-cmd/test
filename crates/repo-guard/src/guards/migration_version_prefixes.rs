use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct MigrationVersionPrefixesReport {
    pub files: usize,
}

pub fn check_migration_version_prefixes(
    repo_root: &Path,
) -> Result<MigrationVersionPrefixesReport, String> {
    let migration_root = repo_root.join("migrations");
    if !migration_root.is_dir() {
        return Err("migration directory is missing: migrations".to_string());
    }

    let mut file_names = Vec::new();
    for entry_result in fs::read_dir(&migration_root)
        .map_err(|error| format!("failed to list migration directory: {error}"))?
    {
        let entry =
            entry_result.map_err(|error| format!("failed to read migration directory: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to read migration file type: {error}"))?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        if has_sql_extension(&file_name) {
            file_names.push(file_name);
        }
    }
    file_names.sort();

    let mut versions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file_name in &file_names {
        let version = migration_version(file_name).ok_or_else(|| {
            format!(
                "migration filename must start with a five digit version prefix: migrations/{file_name}"
            )
        })?;
        versions
            .entry(version.to_string())
            .or_default()
            .push(format!("migrations/{file_name}"));
    }

    for (version, paths) in versions {
        if paths.len() > 1 {
            return Err(format!(
                "duplicate migration version prefix '{version}': {}",
                paths.join(", ")
            ));
        }
    }

    Ok(MigrationVersionPrefixesReport {
        files: file_names.len(),
    })
}

fn has_sql_extension(file_name: &str) -> bool {
    file_name
        .get(file_name.len().saturating_sub(4)..)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(".sql"))
}

fn migration_version(file_name: &str) -> Option<&str> {
    let bytes = file_name.as_bytes();
    if bytes.len() < 11 {
        return None;
    }
    if !bytes[..5].iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes[5] != b'_' {
        return None;
    }
    if !file_name.ends_with(".sql") {
        return None;
    }

    let stem_end = bytes.len() - 4;
    if stem_end <= 6 {
        return None;
    }
    if !bytes[6..stem_end]
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return None;
    }

    Some(&file_name[..5])
}

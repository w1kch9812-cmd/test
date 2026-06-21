use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use repo_guard::guards::migration_version_prefixes::check_migration_version_prefixes;

struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn create(name: &str) -> Result<Self, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path =
            std::env::temp_dir().join(format!("repo-guard-{name}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_migration(&self, name: &str) -> Result<(), Box<dyn Error>> {
        let migrations = self.path.join("migrations");
        fs::create_dir_all(&migrations)?;
        fs::write(migrations.join(name), "-- test migration\n")?;
        Ok(())
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn valid_set_passes() -> Result<(), Box<dyn Error>> {
    let root = TestRoot::create("valid")?;
    root.write_migration("00001_create_user.sql")?;
    root.write_migration("00002_add_listing.sql")?;

    let report = check_migration_version_prefixes(root.path()).map_err(std::io::Error::other)?;

    assert_eq!(2, report.files);
    Ok(())
}

#[test]
fn bad_filename_fails() -> Result<(), Box<dyn Error>> {
    let root = TestRoot::create("bad-filename")?;
    root.write_migration("1_bad.sql")?;

    let error = check_migration_version_prefixes(root.path()).err();

    assert_eq!(
        Some(
            "migration filename must start with a five digit version prefix: migrations/1_bad.sql"
                .to_string()
        ),
        error
    );
    Ok(())
}

#[test]
fn duplicate_prefix_fails() -> Result<(), Box<dyn Error>> {
    let root = TestRoot::create("duplicate")?;
    root.write_migration("00001_create_user.sql")?;
    root.write_migration("00001_add_listing.sql")?;

    let error = check_migration_version_prefixes(root.path()).err();

    assert_eq!(
        Some(
            "duplicate migration version prefix '00001': migrations/00001_add_listing.sql, migrations/00001_create_user.sql"
                .to_string()
        ),
        error
    );
    Ok(())
}

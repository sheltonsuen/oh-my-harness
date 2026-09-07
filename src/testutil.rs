use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn temp_dir(tag: &str) -> PathBuf {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos();
    let dir =
        std::env::temp_dir().join(format!("omh-test-{}-{tag}-{n}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write_skill(repo: &Path, folder: &str, content: &str) {
    let d = repo.join("skills").join(folder);
    fs::create_dir_all(d.join("references")).unwrap();
    fs::write(d.join("SKILL.md"), content).unwrap();
    fs::write(d.join("references").join("x.md"), "ref").unwrap();
}

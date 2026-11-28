//! The files the game reads and writes (data, saves, config.txt, recorded
//! demos, screenshots). Natively these are `std::fs` calls. In the browser
//! (wasm32) there is no file system: the files live in memory, filled by
//! the page at the start (`insert`), and every change is passed to a hook
//! (`set_write_hook`) so the page can keep it (IndexedDB).

use std::io;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(p)
    }
    pub fn write(p: &Path, data: &[u8]) -> io::Result<()> {
        std::fs::write(p, data)
    }
    pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        std::fs::remove_file(p)
    }
    pub fn create_dir_all(p: &Path) -> io::Result<()> {
        std::fs::create_dir_all(p)
    }
    pub fn exists(p: &Path) -> bool {
        p.exists()
    }
    pub fn is_dir(p: &Path) -> bool {
        p.is_dir()
    }
    pub fn list_dir(p: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(std::fs::read_dir(p)?.flatten().map(|e| e.path()).collect())
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// Called with the path and the new contents (None: removed).
    pub type WriteHook = Box<dyn Fn(&str, Option<&[u8]>)>;

    thread_local! {
        static FILES: RefCell<BTreeMap<String, Vec<u8>>> = RefCell::new(BTreeMap::new());
        static HOOK: RefCell<Option<WriteHook>> = RefCell::new(None);
    }

    /// "/KEEN4/saves/../x" -> "/KEEN4/x" (no file system to resolve it).
    pub fn key(p: &Path) -> String {
        let mut parts: Vec<String> = Vec::new();
        for c in p.components() {
            use std::path::Component::*;
            match c {
                Normal(s) => parts.push(s.to_string_lossy().into_owned()),
                ParentDir => {
                    parts.pop();
                }
                _ => {}
            }
        }
        format!("/{}", parts.join("/"))
    }

    fn notify(k: &str, data: Option<&[u8]>) {
        HOOK.with(|h| {
            if let Some(h) = &*h.borrow() {
                h(k, data);
            }
        });
    }

    fn not_found(p: &Path) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, format!("{}: not found", p.display()))
    }

    pub fn insert(p: &Path, data: Vec<u8>) {
        FILES.with(|f| f.borrow_mut().insert(key(p), data));
    }
    pub fn set_write_hook(h: WriteHook) {
        HOOK.with(|x| *x.borrow_mut() = Some(h));
    }
    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        FILES.with(|f| f.borrow().get(&key(p)).cloned()).ok_or_else(|| not_found(p))
    }
    pub fn write(p: &Path, data: &[u8]) -> io::Result<()> {
        let k = key(p);
        FILES.with(|f| f.borrow_mut().insert(k.clone(), data.to_vec()));
        notify(&k, Some(data));
        Ok(())
    }
    pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
        let d = FILES.with(|f| f.borrow_mut().remove(&key(from))).ok_or_else(|| not_found(from))?;
        notify(&key(from), None);
        write(to, &d)
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        let k = key(p);
        FILES.with(|f| f.borrow_mut().remove(&k)).ok_or_else(|| not_found(p))?;
        notify(&k, None);
        Ok(())
    }
    pub fn create_dir_all(_p: &Path) -> io::Result<()> {
        Ok(())
    }
    pub fn exists(p: &Path) -> bool {
        let k = key(p);
        FILES.with(|f| f.borrow().contains_key(&k)) || is_dir(p)
    }
    /// A directory exists while a file is inside it.
    pub fn is_dir(p: &Path) -> bool {
        let prefix = format!("{}/", key(p).trim_end_matches('/'));
        FILES.with(|f| f.borrow().range(prefix.clone()..).next().is_some_and(|(k, _)| k.starts_with(&prefix)))
    }
    pub fn list_dir(p: &Path) -> io::Result<Vec<PathBuf>> {
        let prefix = format!("{}/", key(p).trim_end_matches('/'));
        let mut out: Vec<PathBuf> = FILES.with(|f| {
            f.borrow()
                .range(prefix.clone()..)
                .take_while(|(k, _)| k.starts_with(&prefix))
                .map(|(k, _)| PathBuf::from(&prefix).join(k[prefix.len()..].split('/').next().unwrap_or("")))
                .collect()
        });
        out.dedup();
        Ok(out)
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::{insert, set_write_hook, WriteHook};

pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    imp::read(p.as_ref())
}
pub fn write(p: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
    imp::write(p.as_ref(), data.as_ref())
}
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    imp::rename(from.as_ref(), to.as_ref())
}
pub fn remove_file(p: impl AsRef<Path>) -> io::Result<()> {
    imp::remove_file(p.as_ref())
}
pub fn create_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
    imp::create_dir_all(p.as_ref())
}
pub fn exists(p: impl AsRef<Path>) -> bool {
    imp::exists(p.as_ref())
}
pub fn is_dir(p: impl AsRef<Path>) -> bool {
    imp::is_dir(p.as_ref())
}
/// The entries of a directory.
pub fn list_dir(p: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    imp::list_dir(p.as_ref())
}

pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Seconds and nanoseconds since 1970 (`SystemTime::now` panics in the
/// browser: there it is `Date.now()`).
pub fn now_since_epoch() -> std::time::Duration {
    #[cfg(target_arch = "wasm32")]
    {
        std::time::Duration::from_secs_f64(js_sys::Date::now() / 1000.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[test]
    fn native_round_trip() {
        let d = std::env::temp_dir().join(format!("keen-vfs-{}", std::process::id()));
        super::create_dir_all(&d).unwrap();
        let f = d.join("a.txt");
        super::write(&f, b"hi").unwrap();
        assert_eq!(super::read_to_string(&f).unwrap(), "hi");
        assert!(super::list_dir(&d).unwrap().contains(&f));
        super::remove_file(&f).unwrap();
        assert!(!super::exists(&f));
        std::fs::remove_dir_all(&d).ok();
    }
}

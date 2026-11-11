//! Where `tempest repl` keeps its history, worked out by hand, as each
//! platform keeps an application's data: `$XDG_DATA_HOME/tempest`, or
//! `~/.local/share/tempest`, on Linux and other Unix systems;
//! `~/Library/Application Support/tempest` on macOS; and
//! `%APPDATA%\tempest` on Windows. The file is `repl-history.txt`.
//!
//! What's typed may be private, so on Unix the directory is made
//! owner-only (0700) and the file owner-only (0600), and a file found
//! readable by others is made owner-only again. A path that's there but
//! isn't a plain file, as a link, isn't used.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The history file's name in its directory.
const FILE: &str = "repl-history.txt";

/// The history file's path, from the environment, if the platform's
/// variables say where; `None` if they don't.
pub fn path(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let set = |name: &str| env(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let dir = if cfg!(windows) {
        set("APPDATA")?
    } else if cfg!(target_os = "macos") {
        set("HOME")?.join("Library").join("Application Support")
    } else {
        match set("XDG_DATA_HOME").filter(|d| d.is_absolute()) {
            Some(dir) => dir,
            None => set("HOME")?.join(".local").join("share"),
        }
    };
    Some(dir.join("tempest").join(FILE))
}

/// Readies the history file at `path`: its directory made if need be,
/// and the file itself, empty if it's new, both owner-only on Unix.
pub fn prepare(path: &Path) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(dir)?;
    }
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(io::Error::other("it isn't a plain file"));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut options = fs::OpenOptions::new();
            options.append(true).create(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            options.open(path)?;
        }
        Err(e) => return Err(e),
    }
    owner_only(path)
}

/// Makes a file readable and writable by its owner alone, on Unix.
pub fn owner_only(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_platforms_data_directory() {
        let env = |vars: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                (vars.iter())
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| v.to_string())
            }
        };
        let none = env(&[]);
        assert_eq!(path(&none), None);
        if cfg!(windows) {
            let found = path(&env(&[("APPDATA", r"C:\Users\a\AppData\Roaming")]));
            assert!(found.unwrap().ends_with(r"tempest\repl-history.txt"));
        } else if cfg!(target_os = "macos") {
            let found = path(&env(&[("HOME", "/Users/a")]));
            let expected = "/Users/a/Library/Application Support/tempest/repl-history.txt";
            assert_eq!(found, Some(PathBuf::from(expected)));
        } else {
            let home = env(&[("HOME", "/home/a")]);
            let expected = "/home/a/.local/share/tempest/repl-history.txt";
            assert_eq!(path(&home), Some(PathBuf::from(expected)));
            let xdg = env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "/data")]);
            let expected = "/data/tempest/repl-history.txt";
            assert_eq!(path(&xdg), Some(PathBuf::from(expected)));
            // A relative XDG_DATA_HOME is ignored, as the spec says.
            let relative = env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "data")]);
            let expected = "/home/a/.local/share/tempest/repl-history.txt";
            assert_eq!(path(&relative), Some(PathBuf::from(expected)));
        }
    }

    #[cfg(unix)]
    #[test]
    fn makes_the_file_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("tempest-history-{}", std::process::id()));
        let path = dir.join("tempest").join(FILE);
        prepare(&path).unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        // One readable by others is made owner-only again.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        prepare(&path).unwrap();
        assert_eq!(mode(&path), 0o600);
        // A link isn't followed.
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(prepare(&link).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}

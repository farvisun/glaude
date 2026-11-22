//! The agent host: a background process (`glaude --host`) that owns the
//! `claude` / `codex` processes, so they keep working while the window is
//! closed. The app talks to it over a local socket; see `docs/plan-t3-features.md`.

pub mod client;
pub mod server;
pub mod state;

use std::io;
use std::path::PathBuf;

use tokio::io::{AsyncRead, AsyncWrite};

/// Bumped when requests or replies change incompatibly.
pub const PROTO: u32 = 1;

pub type Reader = Box<dyn AsyncRead + Unpin + Send>;
pub type Writer = Box<dyn AsyncWrite + Unpin + Send>;

/// The app's data folder (same as Tauri's app data dir).
pub fn data_dir() -> PathBuf {
    // Tests run their own host in a temporary folder.
    if let Some(d) = std::env::var_os("GLAUDE_HOST_DIR") {
        return PathBuf::from(d);
    }
    dirs::data_dir()
        .unwrap_or_else(glaude_core::env::home_dir)
        .join("com.farvisun.glaude")
}

pub fn token_path() -> PathBuf {
    data_dir().join("host.token")
}

pub fn log_path() -> PathBuf {
    data_dir().join("host.log")
}

/// Identifies the binary: a host started from an older build is replaced
/// once it has nothing running.
pub fn exe_stamp() -> String {
    std::env::current_exe()
        .and_then(std::fs::metadata)
        .map(|m| {
            let t = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("{t}-{}", m.len())
        })
        .unwrap_or_default()
}

#[cfg(unix)]
pub fn socket_path() -> PathBuf {
    let p = data_dir().join("host.sock");
    // sun_path is ~104 bytes on macOS.
    if p.as_os_str().len() < 100 {
        p
    } else {
        std::env::temp_dir().join(format!("glaude-{}.sock", unsafe { libc::getuid() }))
    }
}

#[cfg(windows)]
pub fn pipe_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    let user: String = user
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    format!(r"\\.\pipe\glaude-host-{user}")
}

pub async fn connect() -> io::Result<(Reader, Writer)> {
    #[cfg(unix)]
    {
        let s = tokio::net::UnixStream::connect(socket_path()).await?;
        let (r, w) = s.into_split();
        Ok((Box::new(r), Box::new(w)))
    }
    #[cfg(windows)]
    {
        let c = tokio::net::windows::named_pipe::ClientOptions::new().open(pipe_name())?;
        let (r, w) = tokio::io::split(c);
        Ok((Box::new(r), Box::new(w)))
    }
}

pub struct Listener {
    #[cfg(unix)]
    inner: tokio::net::UnixListener,
    #[cfg(windows)]
    next: tokio::net::windows::named_pipe::NamedPipeServer,
}

impl Listener {
    pub fn bind() -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = socket_path();
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let _ = std::fs::remove_file(&path);
            let inner = tokio::net::UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            Ok(Self { inner })
        }
        #[cfg(windows)]
        {
            let next = tokio::net::windows::named_pipe::ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .create(pipe_name())?;
            Ok(Self { next })
        }
    }

    pub async fn accept(&mut self) -> io::Result<(Reader, Writer)> {
        #[cfg(unix)]
        {
            let (s, _) = self.inner.accept().await?;
            let (r, w) = s.into_split();
            Ok((Box::new(r), Box::new(w)))
        }
        #[cfg(windows)]
        {
            self.next.connect().await?;
            let fresh = tokio::net::windows::named_pipe::ServerOptions::new()
                .reject_remote_clients(true)
                .create(pipe_name())?;
            let conn = std::mem::replace(&mut self.next, fresh);
            let (r, w) = tokio::io::split(conn);
            Ok((Box::new(r), Box::new(w)))
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(socket_path());
        }
    }
}

/// A random secret only this user can read; clients must present it.
pub fn new_token() -> io::Result<String> {
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let path = token_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, &path)?;
    Ok(token)
}

pub fn read_token() -> io::Result<String> {
    Ok(std::fs::read_to_string(token_path())?.trim().to_string())
}

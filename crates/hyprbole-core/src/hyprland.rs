use std::env;
use std::fmt;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[derive(Debug)]
pub enum HyprlandIpcError {
    MissingEnvironment(&'static str),
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    Write(std::io::Error),
    Shutdown(std::io::Error),
    Read(std::io::Error),
    Rejected {
        command: String,
        reply: String,
    },
}

impl fmt::Display for HyprlandIpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironment(name) => write!(f, "{name} is not set"),
            Self::Connect { path, source } => write!(f, "connect {}: {source}", path.display()),
            Self::Write(source) => write!(f, "write command: {source}"),
            Self::Shutdown(source) => write!(f, "shutdown write side: {source}"),
            Self::Read(source) => write!(f, "read reply: {source}"),
            Self::Rejected { command, reply } => {
                write!(f, "Hyprland rejected `{command}`: {}", reply.trim())
            }
        }
    }
}

impl std::error::Error for HyprlandIpcError {}

pub type HyprlandIpcResult<T> = Result<T, HyprlandIpcError>;

#[derive(Debug, Clone)]
pub struct HyprlandIpc {
    socket_path: PathBuf,
}

impl HyprlandIpc {
    pub fn from_env() -> HyprlandIpcResult<Self> {
        let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or(HyprlandIpcError::MissingEnvironment("XDG_RUNTIME_DIR"))?;
        let instance_signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE").ok_or(
            HyprlandIpcError::MissingEnvironment("HYPRLAND_INSTANCE_SIGNATURE"),
        )?;

        Ok(Self {
            socket_path: runtime_dir
                .join("hypr")
                .join(instance_signature)
                .join(".socket.sock"),
        })
    }

    pub fn send_command(&self, command: &str) -> HyprlandIpcResult<String> {
        let mut stream =
            UnixStream::connect(&self.socket_path).map_err(|source| HyprlandIpcError::Connect {
                path: self.socket_path.clone(),
                source,
            })?;

        let mut request = String::with_capacity(command.len() + 1);
        request.push_str(command);
        request.push('\n');
        stream
            .write_all(request.as_bytes())
            .map_err(HyprlandIpcError::Write)?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(HyprlandIpcError::Shutdown)?;

        let mut reply = String::new();
        stream
            .read_to_string(&mut reply)
            .map_err(HyprlandIpcError::Read)?;
        Ok(reply)
    }

    pub fn eval_ok(&self, lua: &str) -> HyprlandIpcResult<()> {
        let command = format!("eval {lua}");
        self.command_ok(&command)
    }

    pub fn command_ok(&self, command: &str) -> HyprlandIpcResult<()> {
        let reply = self.send_command(&command)?;
        if reply.trim() == "ok" {
            Ok(())
        } else {
            Err(HyprlandIpcError::Rejected {
                command: command.to_string(),
                reply,
            })
        }
    }
}

pub fn register_shell_mouse_binds() -> HyprlandIpcResult<()> {
    let ipc = HyprlandIpc::from_env()?;
    for lua in [
        r#"hl.unbind("SUPER + mouse:272")"#,
        r#"hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })"#,
        r#"hl.unbind("SUPER + mouse:273")"#,
        r#"hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })"#,
    ] {
        ipc.eval_ok(lua)?;
    }
    Ok(())
}

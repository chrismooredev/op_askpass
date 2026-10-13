use std::fmt;
use std::io;
use std::io::Read as _;


/// states:
/// 1: uninitialized: must sign in, op whoami fails (no session key/secret)
/// 2: initialized: can not sign in, op whoami succeeds
/// 3: expired: must sign in, op whoami succeeds
#[derive(Clone)]
pub struct OnepassSession {
    key: String,
    secret: String,
}

impl OnepassSession {
    pub fn from_env() -> impl Iterator<Item = OnepassSession> {
        const OP_SESSION: &str = "OP_SESSION_";
        std::env::vars_os()
            // implicit assumption that the Onepass Session var is unicode... feels safe enough
            .filter_map(|(k,v)| match (k.into_string(), v.into_string()) {
                (Ok(uk), Ok(uv)) => Some((uk, uv)),
                _ => None,
            })
            .filter_map(|(k, v)| {
                k.strip_prefix(OP_SESSION).map(|key|
                    OnepassSession {
                        key: key.to_string(),
                        secret: v
                    }
                )
            })
    }
    /// Tests the held session info for liveness/validity. Cannot test for time remaining.
    pub fn is_valid(&self) -> Result<OnepassCli, InvalidOnepassSession> {
        let mut who = std::process::Command::new("op").args(["signin", "--session", &self.secret])
            .stdin(std::process::Stdio::null()) // we will never put in a password ourselves
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn().expect("Unable to start 1Password CLI (`op signin`) process");

        let exit_status = who.wait().expect("Failed on waiting for `op whoami` to finish");

        if exit_status.success() {
            Ok(OnepassCli::Session(self.clone()))
        } else {
            let mut stderr = String::new();
            who.stderr.unwrap().read_to_string(&mut stderr).expect("Unable to read stderr from failed process");
            Err(InvalidOnepassSession(stderr.trim().to_string()))
        }
    }
}
impl fmt::Debug for OnepassSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnepassSession")
            .field("key", &self.key)
            .field("secret", &"<redacted>")
            .finish()
    }
}


/// A validated means of invoking the 1Password CLI.
///
/// `Session` wraps an explicit `OP_SESSION_*` environment variable, passed via
/// `op`'s `--session` flag. `Agent` indicates no such variable was found, but
/// `op` is still able to authenticate on its own (e.g. via the native
/// desktop app integration/"op agent", commonly used on macOS) - so commands
/// are run without a `--session` flag, relying on `op` to handle auth itself.
#[derive(Debug, Clone)]
pub enum OnepassCli {
    Session(OnepassSession),
    Agent,
}

#[derive(Debug, Clone)]
pub struct InvalidOnepassSession(String);
impl std::error::Error for InvalidOnepassSession {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.0
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}
impl fmt::Display for InvalidOnepassSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}


impl OnepassCli {
    /// Returns a `OnepassCLI` struct with a valid session or returns an Err
    pub fn from_env() -> Result<OnepassCli, InvalidOnepassSession> {
        let c: Vec<_> = OnepassSession::from_env().collect();
        match c.split_last() {
            None => Err(InvalidOnepassSession("No sessions initalized".to_string())),
            Some((l, s)) => {
                for k in s {
                    if let Ok(cli) = k.is_valid() {
                        return Ok(cli);
                    }
                }
                l.is_valid()
            }
        }
    }

    /// Queries 1Password CLI for the existing logged in user.
    pub fn from_command() -> Result<OnepassCli, InvalidOnepassSession> {
        if let Ok(v) = Self::from_env() {
            return Ok(v);
        }

        // try to get a better error message

        // note that `op whoami` still returns exit code zero after session expiration (we still are an identity! it just must be refreshed)
        let who = std::process::Command::new("op").args(["whoami"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn().expect("Unable to start 1Password CLI (`op whoami`) process");

        // TODO: reflect true status of uninitialized/logged in/expired in return

        let output = who.wait_with_output().expect("Failed on waiting for `op whoami` to finish");

        let out = String::from_utf8_lossy(output.stdout.trim_ascii()).to_string();
        let err = String::from_utf8_lossy(output.stderr.trim_ascii()).to_string();

        let mut msg = String::new();
        if !out.is_empty() { msg.push_str("stdout: "); msg.push_str(&out); }
        if !err.is_empty() { msg.push_str("stderr: "); msg.push_str(&err); }

        if err.is_empty() && output.status.success() {
            // `op whoami` succeeded without any `OP_SESSION_*` env var being set - this
            // happens when `op` authenticates on its own, e.g. via the native desktop
            // app integration/"op agent" (commonly used on macOS). Treat this as a
            // valid, non-error authentication method rather than failing.
            return Ok(OnepassCli::Agent);
        }

        Err(InvalidOnepassSession(msg))
    }

    pub fn read(&self, op_uri: &str) -> io::Result<String> {
        let mut cmd = std::process::Command::new("op");
        cmd.arg("read").arg(op_uri);
        if let OnepassCli::Session(session) = self {
            cmd.args(["--session", &session.secret]);
        }
        let pwd = cmd.stdout(std::process::Stdio::piped()).spawn()?;
        let pwd_out = pwd.wait_with_output()?;
        if !pwd_out.status.success() {
            return Err(io::Error::other(format!(
                "Failed to read secret: {}",
                String::from_utf8_lossy(pwd_out.stderr.trim_ascii())
            )));
        }
        Ok(String::from_utf8_lossy_owned(pwd_out.stdout))
    }
}

#![warn(clippy::pedantic)]

use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::process::{ExitCode, ExitStatus};
use std::io::{BufReader, IsTerminal};

use regex::Regex;
use ssh2_config::{HostParams, ParseRule, SshConfig};
use sysinfo::{KillError, Pid};

use crate::onepass::OnepassCli;

mod onepass;

pub struct HostConfig(pub HostParams);
impl HostConfig {
    pub fn query_first_ignored(&self, key: &str) -> Option<&str> {
        self.0.ignored_fields.iter()
            .find_map(|(k, v)| k.eq_ignore_ascii_case(key).then_some(v.first()).flatten())
            .map(String::as_str)
    }
}

fn ssh_properties_for(host: &str) -> HostConfig {
    let home = dirs::home_dir().expect("unable to resolve home directory");
    let config = home.join(".ssh").join("config");
    let mut f = File::open(config)
        .map(BufReader::new)
        .expect("unable to open ssh config");

    // `ssh -G $host` does not emit unknown fields, so we must use an independant parser
    let sc = SshConfig::default()
        // ALLOW_UNKNOWN_FIELDS for Our OnepassURI and... Match exec..
        .parse(&mut f, ParseRule::ALLOW_UNKNOWN_FIELDS)
        .expect("unable to parse ssh config");

    HostConfig(sc.query(host))
}

#[derive(Debug)]
struct ParentProc {
    exe: OsString,
    pid: Pid,
    args: Vec<OsString>,
}
impl ParentProc {
    fn is_ssh(&self) -> bool {
        self.exe == "ssh"
    }
    fn verbosity_flags(args: &[OsString]) -> usize {
        let mut v = 0;
        for arg in args {
            let s = arg.to_string_lossy();
            if s.chars().nth(0) != Some('-') { continue; } // this is a flag var
            if s.chars().nth(1) == Some('-') { continue; } // no --verbose flag

            let vees = s.chars()
                .skip(1)
                .take_while(char::is_ascii_alphabetic)
                .filter(|c| *c == 'v')
                .count();

            v += vees;
        }
        v
    }
}

fn process_parents() -> Vec<ParentProc> {
    let mut par_procs = Vec::new();
    let sys = sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::everything()));

    let mut pid_res = Some(sysinfo::get_current_pid().expect("process pid unavailable"));

    while let Some(pid) = pid_res {
        let Some(proc) = sys.process(pid) else { break; };
        let exe = proc.exe().unwrap_or(Path::new("<error>")); //expect("parent process exe path unavailable");
        let name = exe.file_prefix().expect("parent process exe filename unavailable");
        par_procs.push(ParentProc {
            exe: name.to_os_string(),
            pid,
            args: proc.cmd().to_owned(),
        });

        pid_res = proc.parent();
    }

    par_procs
}

fn send_pid_kill_and_wait(pid: Pid) -> Result<Option<ExitStatus>, KillError> {
    let sys = sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::everything()));
    sys.process(pid).unwrap().kill_and_wait()
}

fn main() -> ExitCode {
    let parents = process_parents();
    let par = &parents[1];

    let stdin_tty = std::io::stdin().is_terminal();
    // eprintln!("Stdin terminal: {:?}", stdin_tty);

    let v = parents.iter()
        .find(|p| p.is_ssh())
        .map_or(0, |p| ParentProc::verbosity_flags(&p.args));

    if std::env::var("OP_ASKPASS_VERBOSE").is_ok() || v > 0 {
        eprintln!("Process stacks:");
        parents.iter()
            .enumerate()
            .for_each(|(i, p)| {
                eprintln!("[{i}]: {p:?}");
            });
    }

    if std::env::args().any(|p| p == "--is-ssh") {
        let is_ssh = parents.iter().skip(1).take(2)
            .any(ParentProc::is_ssh);

        if is_ssh { ExitCode::SUCCESS } else { ExitCode::FAILURE }
    } else if !par.is_ssh() || std::env::args().any(|p| p == "--env=pwsh") {
        // eprintln!("Not running in SSH - printing pwsh env");
        print!("$env:SSH_ASKPASS_REQUIRE='force'; ");
        println!("$env:SSH_ASKPASS='{}';", std::env::current_exe().unwrap().display());
        ExitCode::SUCCESS
    } else {
        let mut args = std::env::args().collect::<Vec<_>>();

        let _ssh = args.remove(0);
        // eprintln!("running in ssh: {:?}", args);
        let first = args.first().expect("no arguments provided from ssh exe");

        {
            // Known password prompt regexes, used primarily for the host field to lookup the 1Password URI
            let regexes = [
                Regex::new(r"\((?P<user>[^@]+)@(?P<host>[^)]+)\) Password:").unwrap(), // "(user@host) Password:"
                Regex::new(r"(?P<user>[^@]+)@(?P<host>[^)]+)'s password: ").unwrap(), // "user@host's password:"
            ];

            let opt_userhost = regexes.into_iter()
                .find_map(|re| re.captures(first)) // Find our matching prompt regex, if any
                .map(|caps| (caps.name("user").unwrap().as_str(), caps.name("host").unwrap().as_str())); // Extract the host field

            match opt_userhost {
                None => eprintln!("op_askpass: unable to fetch user@host from unknown prompt: {first:?}"),
                Some((user, host)) => {
                    if v >= 1 { eprintln!("Parsing SSH config"); }
                    let conf = ssh_properties_for(host); // fetch op:// uri from ssh config
                    match conf.0.user {
                        None => eprintln!("op_askpass: no user specified in config, requiring manual input"),
                        Some(u) if u != user => {
                            eprintln!("op_askpass: different user specified in config than request, requiring manual input");
                        },
                        Some(_) => match conf.query_first_ignored("OnepassUri") {
                            None => eprintln!("op_askpass: no 1Password URI found in SSH config for host {host:?}"),
                            Some(uri) => {
                                if v >= 1 { eprintln!("Checking 1Password CLI status"); }
                                let op_status = OnepassCli::from_command();
                                if v >= 1 { eprintln!("Done checking 1Password CLI status"); }
                                match op_status {
                                    Err(err) => eprintln!("op_askpass: {err}"),
                                    Ok(op) => {
                                        if v >= 1 { eprintln!("Reading 1Password CLI secret"); }
                                        let op_secret = op.read(uri);
                                        if v >= 1 { eprintln!("Done reading 1Password CLI secret"); }
                                        match op_secret {
                                            Err(e) => eprintln!("op_askpass: unable to read reference {uri:?}: {e}"),
                                            Ok(val) => {
                                                print!("{val}");
                                                return ExitCode::SUCCESS;
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if stdin_tty {
                let k = rpassword::prompt_password(format_args!("op_askpass: {first}"))
                    .expect("Unable to fetch password from user");

                print!("{k}");
                ExitCode::SUCCESS
            } else {
                eprintln!("op_askpass: Unable to prompt without stdin tty, stopping parent ssh & exiting");
                let _ = send_pid_kill_and_wait(par.pid);
                ExitCode::FAILURE
            }
        }
    }
}

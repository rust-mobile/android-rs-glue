//! This module provides features to pretty-print command execution in the tty.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::process::exit;
use term;

pub struct TermCmd {
    label: String,
    command: Command,
    command_label: Vec<String>,
    inherit_stdout: bool,
}

impl TermCmd {
    #[inline]
    pub fn new<L: Into<String>, S: AsRef<OsStr>>(label: L, program: S) -> TermCmd {
        let command_label = program.as_ref().to_string_lossy().into_owned();

        TermCmd {
            label: label.into(),
            command: {
                let mut cmd = Command::new(program);
                cmd.stderr(Stdio::piped());
                cmd
            },
            command_label: vec![command_label],
            inherit_stdout: false,
        }
    }

    #[inline]
    pub fn inherit_stdout(&mut self) -> &mut TermCmd {
        self.inherit_stdout = true;
        self
    }

    #[inline]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut TermCmd {
        self.command_label.push(arg.as_ref().to_string_lossy().into_owned());
        self.command.arg(arg);
        self
    }

    #[inline]
    pub fn env<K: AsRef<OsStr>, V: AsRef<OsStr>>(&mut self, key: K, val: V) -> &mut TermCmd {
        self.command.env(key, val);
        self
    }

    #[inline]
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut TermCmd {
        self.command.current_dir(dir);
        self
    }

    #[inline]
    pub fn execute(&mut self) {
        self.exec_stdout();
    }

    pub fn exec_stdout(&mut self) -> Vec<u8> {
        if self.inherit_stdout {
            self.command.stdout(Stdio::inherit());
        } else {
            self.command.stdout(Stdio::piped());
        }

        let mut t = term::stdout().unwrap();

        t.fg(term::color::BRIGHT_GREEN).unwrap();
        t.attr(term::Attr::Bold).unwrap();
        writeln!(t, "  Cargo-Apk: {}", self.label).unwrap();
        t.reset().unwrap();

        let output = self.command.output();
        let success = match output.as_ref().map(|o| o.status) {
            Ok(status) if status.success() => true,
            _ => false,
        };

        if success {
            return output.unwrap().stdout;
        }

        t.fg(term::color::RED).unwrap();
        writeln!(t, "Error executing {:?}", self.command_label).unwrap();
        match output.as_ref().map(|o| o.status.code()) {
            Ok(Some(code)) => writeln!(t, "Status code {}", code).unwrap(),
            Ok(None) => writeln!(t, "Interrupted").unwrap(),
            Err(err) => writeln!(t, "{}", err).unwrap(),
        }
        t.reset().unwrap();

        if let Ok(ref output) = output {
            if !self.inherit_stdout {
                writeln!(t, "Stdout\n--------------------").unwrap();
                t.write_all(&output.stdout).unwrap();
            }
            writeln!(t, "Stderr\n--------------------").unwrap();
            t.write_all(&output.stderr).unwrap();
        }

        exit(1);    // TODO: meh, shouldn't exit here
    }
}

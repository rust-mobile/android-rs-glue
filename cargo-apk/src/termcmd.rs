//! This module provides features to pretty-print command execution in the tty.

use std::io;
use std::io::Write;
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
    inherit_stdouterr: bool,
}

impl TermCmd {
    #[inline]
    pub fn new<L: Into<String>, S: AsRef<OsStr>>(label: L, program: S) -> TermCmd {
        let command_label = program.as_ref().to_string_lossy().into_owned();

        TermCmd {
            label: label.into(),
            command: Command::new(program),
            command_label: vec![command_label],
            inherit_stdouterr: false,
        }
    }

    #[inline]
    pub fn inherit_stdouterr(&mut self) -> &mut TermCmd {
        self.inherit_stdouterr = true;
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
        if self.inherit_stdouterr {
            self.command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            self.command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }

        let mut t = term::stdout();

        if let Some(ref mut t) = t {
            let _ = t.fg(term::color::BRIGHT_GREEN);
            let _ = t.attr(term::Attr::Bold);
            writeln!(t, "  Cargo-Apk: {}", self.label).unwrap();
            t.reset().unwrap();
        } else {
            println!("  Cargo-Apk: {}", self.label);
        }

        let output = self.command.output();
        let success = match output.as_ref().map(|o| o.status) {
            Ok(status) if status.success() => true,
            _ => false,
        };

        if success {
            return output.unwrap().stdout;
        }

        if let Some(ref mut t) = t {
            let _ = t.fg(term::color::RED);
            writeln!(t, "Error executing {:?}", self.command_label).unwrap();
            match output.as_ref().map(|o| o.status.code()) {
                Ok(Some(code)) => writeln!(t, "Status code {}", code).unwrap(),
                Ok(None) => writeln!(t, "Interrupted").unwrap(),
                Err(err) => writeln!(t, "{}", err).unwrap(),
            }
            t.reset().unwrap();

            if let Ok(ref output) = output {
                if !self.inherit_stdouterr {
                    writeln!(t, "Stdout\n--------------------").unwrap();
                    t.write_all(&output.stdout).unwrap();
                    writeln!(t, "Stderr\n--------------------").unwrap();
                    t.write_all(&output.stderr).unwrap();
                }
            }

        } else {
            println!("Error executing {:?}", self.command_label);
            match output.as_ref().map(|o| o.status.code()) {
                Ok(Some(code)) => println!("Status code {}", code),
                Ok(None) => println!("Interrupted"),
                Err(err) => println!("{}", err),
            }
            if let Ok(ref output) = output {
                if !self.inherit_stdouterr {
                    println!("Stdout\n--------------------");
                    io::stdout().write_all(&output.stdout).unwrap();
                    println!("Stderr\n--------------------");
                    io::stdout().write_all(&output.stderr).unwrap();
                }
            }
        }

        exit(1);    // TODO: meh, shouldn't exit here
    }
}

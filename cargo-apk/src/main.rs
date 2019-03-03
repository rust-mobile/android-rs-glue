extern crate cargo;
extern crate clap;
extern crate rustc_serialize;
extern crate term;
extern crate toml;

#[macro_use]
extern crate failure;

use std::path::PathBuf;

use cargo::core::compiler::{BuildConfig, CompileMode, MessageFormat};
use cargo::core::Workspace;
use cargo::ops::{CompileFilter, CompileOptions, Packages};
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::process_builder::process;
use cargo::util::Config as CargoConfig;
use cargo::CargoResult;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

mod config;
mod ops;

fn main() {
    let mut cargo_config = CargoConfig::default().unwrap();

    let args = match cli().get_matches_safe() {
        Ok(args) => args,
        Err(err) => cargo::exit_with_error(err.into(), &mut *cargo_config.shell()),
    };

    let (command, subcommand_args) = match args.subcommand() {
        (command, Some(subcommand_args)) => (command, subcommand_args),
        _ => {
            drop(cli().print_help());
            return;
        }
    };
    assert_eq!(command, "apk");

    let arg_target_dir = &subcommand_args.value_of_path("target-dir", &cargo_config);

    cargo_config
        .configure(
            args.occurrences_of("verbose") as u32,
            if args.is_present("quiet") {
                Some(true)
            } else {
                None
            },
            &args.value_of("color").map(|s| s.to_string()),
            args.is_present("frozen"),
            args.is_present("locked"),
            arg_target_dir,
            &args
                .values_of_lossy("unstable-features")
                .unwrap_or_default(),
        )
        .unwrap();

    let (command, subcommand_args) = match subcommand_args.subcommand() {
        (command, Some(subcommand_args)) => (command, subcommand_args),
        _ => {
            drop(cli().print_help());
            return;
        }
    };

    let err = match command {
        "build" => execute_build(&subcommand_args, &cargo_config),
        "install" => execute_install(&subcommand_args, &cargo_config),
        "run" => execute_run(&subcommand_args, &cargo_config),
        "logcat" => execute_logcat(&subcommand_args, &cargo_config),
        _ => cargo::exit_with_error(
            format_err!("Expected `build`, `install`, `run`, or `logcat`, got {:?}", (command, subcommand_args)).into(),
            &mut *cargo_config.shell(),
        ),
    };

    match err {
        Ok(_) => (),
        Err(err) => cargo::exit_with_error(err, &mut *cargo_config.shell()),
    }
}

fn cli() -> App<'static, 'static> {
    App::new("cargo-apk")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::VersionlessSubcommands,
            AppSettings::AllowExternalSubcommands,
        ])
        .subcommand(
            SubCommand::with_name("apk")
                .subcommands(vec![
                    cli_build(),
                    cli_install(),
                    cli_run(),
                    cli_logcat(),
                ])
        )
        .arg(opt("verbose", "Verbose output"))
}

fn cli_build() -> App<'static, 'static> {
    SubCommand::with_name("build")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::DontCollapseArgsInUsage,
        ])
        .alias("b")
        .about("Compile a local package and all of its dependencies")
        .arg_package_spec(
            "Package to build (see `cargo help pkgid`)",
            "Build all packages in the workspace",
            "Exclude packages from the build",
        )
        .arg_jobs()
        .arg_targets_all(
            "Build only this package's library",
            "Build only the specified binary",
            "Build all binaries",
            "Build only the specified example",
            "Build all examples",
            "Build only the specified test target",
            "Build all tests",
            "Build only the specified bench target",
            "Build all benches",
            "Build all targets",
        )
        .arg_release("Build artifacts in release mode, with optimizations")
        .arg_features()
        .arg_target_triple("Build for the target triple")
        .arg_target_dir()
        .arg(opt("out-dir", "Copy final artifacts to this directory").value_name("PATH"))
        .arg_manifest_path()
        .arg_message_format()
        .arg_build_plan()
        .after_help(
            "\
All packages in the workspace are built if the `--all` flag is supplied. The
`--all` flag is automatically assumed for a virtual manifest.
Note that `--exclude` has to be specified in conjunction with the `--all` flag.

Compilation can be configured via the use of profiles which are configured in
the manifest. The default profile for this command is `dev`, but passing
the --release flag will use the `release` profile instead.
",
        )
}

fn cli_install() -> App<'static, 'static> {
    SubCommand::with_name("install")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::DontCollapseArgsInUsage,
        ])
        .about("Install a Rust binary")
        .arg(Arg::with_name("crate").empty_values(false).multiple(true))
        .arg(
            opt("version", "Specify a version to install from crates.io")
                .alias("vers")
                .value_name("VERSION"),
        )
        .arg(opt("git", "Git URL to install the specified crate from").value_name("URL"))
        .arg(opt("branch", "Branch to use when installing from git").value_name("BRANCH"))
        .arg(opt("tag", "Tag to use when installing from git").value_name("TAG"))
        .arg(opt("rev", "Specific commit to use when installing from git").value_name("SHA"))
        .arg(opt("path", "Filesystem path to local crate to install").value_name("PATH"))
        .arg(opt(
            "list",
            "list all installed packages and their versions",
        ))
        .arg_jobs()
        .arg(opt("force", "Force overwriting existing crates or binaries").short("f"))
        .arg_features()
        .arg(opt("debug", "Build in debug mode instead of release mode"))
        .arg_targets_bins_examples(
            "Install only the specified binary",
            "Install all binaries",
            "Install only the specified example",
            "Install all examples",
        )
        .arg_target_triple("Build for the target triple")
        .arg(opt("root", "Directory to install packages into").value_name("DIR"))
        .arg(opt("registry", "Registry to use").value_name("REGISTRY"))
        .after_help(
            "\
This command manages Cargo's local set of installed binary crates. Only packages
which have [[bin]] targets can be installed, and all binaries are installed into
the installation root's `bin` folder. The installation root is determined, in
order of precedence, by `--root`, `$CARGO_INSTALL_ROOT`, the `install.root`
configuration key, and finally the home directory (which is either
`$CARGO_HOME` if set or `$HOME/.cargo` by default).

There are multiple sources from which a crate can be installed. The default
location is crates.io but the `--git` and `--path` flags can change this source.
If the source contains more than one package (such as crates.io or a git
repository with multiple crates) the `<crate>` argument is required to indicate
which crate should be installed.

Crates from crates.io can optionally specify the version they wish to install
via the `--vers` flags, and similarly packages from git repositories can
optionally specify the branch, tag, or revision that should be installed. If a
crate has multiple binaries, the `--bin` argument can selectively install only
one of them, and if you'd rather install examples the `--example` argument can
be used as well.

By default cargo will refuse to overwrite existing binaries. The `--force` flag
enables overwriting existing binaries. Thus you can reinstall a crate with
`cargo install --force <crate>`.

Omitting the <crate> specification entirely will
install the crate in the current directory. That is, `install` is equivalent to
the more explicit `install --path .`.  This behaviour is deprecated, and no
longer supported as of the Rust 2018 edition.

If the source is crates.io or `--git` then by default the crate will be built
in a temporary target directory.  To avoid this, the target directory can be
specified by setting the `CARGO_TARGET_DIR` environment variable to a relative
path.  In particular, this can be useful for caching build artifacts on
continuous integration systems.",
        )
}

fn cli_run() -> App<'static, 'static> {
    SubCommand::with_name("run")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::DontCollapseArgsInUsage,
        ])
        .alias("r")
        .setting(AppSettings::TrailingVarArg)
        .about("Run the main binary of the local package (src/main.rs)")
        .arg(Arg::with_name("args").multiple(true))
        .arg_targets_bin_example(
            "Name of the bin target to run",
            "Name of the example target to run",
        )
        .arg_package("Package with the target to run")
        .arg_jobs()
        .arg_release("Build artifacts in release mode, with optimizations")
        .arg_features()
        .arg_target_triple("Build for the target triple")
        .arg_target_dir()
        .arg_manifest_path()
        .arg_message_format()
        .after_help(
            "\
If neither `--bin` nor `--example` are given, then if the package only has one
bin target it will be run. Otherwise `--bin` specifies the bin target to run,
and `--example` specifies the example target to run. At most one of `--bin` or
`--example` can be provided.

All the arguments following the two dashes (`--`) are passed to the binary to
run. If you're passing arguments to both Cargo and the binary, the ones after
`--` go to the binary, the ones before go to Cargo.
",
        )
}

fn cli_logcat() -> App<'static, 'static> {
    SubCommand::with_name("logcat")
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::DontCollapseArgsInUsage,
        ])
        .alias("r")
        .about("Print Android log")
        .arg_message_format()
}

pub fn execute_build(options: &ArgMatches, cargo_config: &CargoConfig) -> cargo::CliResult {
    let root_manifest = find_root_manifest_for_wd(&options.manifest_path(cargo_config))?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let mut android_config = config::load(
        &workspace,
        &options.value_of("package").map(|s| s.to_owned()),
    )?;
    android_config.release = options.is_present("release");

    ops::build(&workspace, &android_config, &options)?;
    Ok(())
}

pub fn execute_install(options: &ArgMatches, cargo_config: &CargoConfig) -> cargo::CliResult {
    let root_manifest = find_root_manifest_for_wd(&options.manifest_path(cargo_config))?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let mut android_config = config::load(
        &workspace,
        &options.value_of("package").map(|s| s.to_owned()),
    )?;
    android_config.release = options.is_present("release");

    ops::install(&workspace, &android_config, &options)?;
    Ok(())
}

pub fn execute_run(options: &ArgMatches, cargo_config: &CargoConfig) -> cargo::CliResult {
    let root_manifest = find_root_manifest_for_wd(&options.manifest_path(cargo_config))?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let mut android_config = config::load(
        &workspace,
        &options.value_of("package").map(|s| s.to_owned()),
    )?;
    android_config.release = options.is_present("release");

    ops::run(&workspace, &android_config, &options)?;
    Ok(())
}

pub fn execute_logcat(options: &ArgMatches, cargo_config: &CargoConfig) -> cargo::CliResult {
    let root_manifest = find_root_manifest_for_wd(&options.manifest_path(cargo_config))?;

    let workspace = Workspace::new(&root_manifest, &cargo_config)?;

    let android_config = config::load(
        &workspace,
        &options.value_of("package").map(|s| s.to_owned()),
    )?;

    drop(writeln!(
        workspace.config().shell().err(),
        "Starting logcat"
    ));
    let adb = android_config.sdk_path.join("platform-tools/adb");
    process(&adb).arg("logcat").exec()?;

    Ok(())
}

// Copied from `cargo/command_prelude.rs`

pub trait AppExt: Sized {
    fn _arg(self, arg: Arg<'static, 'static>) -> Self;

    fn arg_package_spec(
        self,
        package: &'static str,
        all: &'static str,
        exclude: &'static str,
    ) -> Self {
        self.arg_package_spec_simple(package)
            ._arg(opt("all", all))
            ._arg(multi_opt("exclude", "SPEC", exclude))
    }

    fn arg_package_spec_simple(self, package: &'static str) -> Self {
        self._arg(multi_opt("package", "SPEC", package).short("p"))
    }

    fn arg_package(self, package: &'static str) -> Self {
        self._arg(opt("package", package).short("p").value_name("SPEC"))
    }

    fn arg_jobs(self) -> Self {
        self._arg(
            opt("jobs", "Number of parallel jobs, defaults to # of CPUs")
                .short("j")
                .value_name("N"),
        )
    }

    fn arg_targets_all(
        self,
        lib: &'static str,
        bin: &'static str,
        bins: &'static str,
        example: &'static str,
        examples: &'static str,
        test: &'static str,
        tests: &'static str,
        bench: &'static str,
        benches: &'static str,
        all: &'static str,
    ) -> Self {
        self.arg_targets_lib_bin(lib, bin, bins)
            ._arg(multi_opt("example", "NAME", example))
            ._arg(opt("examples", examples))
            ._arg(multi_opt("test", "NAME", test))
            ._arg(opt("tests", tests))
            ._arg(multi_opt("bench", "NAME", bench))
            ._arg(opt("benches", benches))
            ._arg(opt("all-targets", all))
    }

    fn arg_targets_lib_bin(self, lib: &'static str, bin: &'static str, bins: &'static str) -> Self {
        self._arg(opt("lib", lib))
            ._arg(multi_opt("bin", "NAME", bin))
            ._arg(opt("bins", bins))
    }

    fn arg_targets_bins_examples(
        self,
        bin: &'static str,
        bins: &'static str,
        example: &'static str,
        examples: &'static str,
    ) -> Self {
        self._arg(multi_opt("bin", "NAME", bin))
            ._arg(opt("bins", bins))
            ._arg(multi_opt("example", "NAME", example))
            ._arg(opt("examples", examples))
    }

    fn arg_targets_bin_example(self, bin: &'static str, example: &'static str) -> Self {
        self._arg(multi_opt("bin", "NAME", bin))
            ._arg(multi_opt("example", "NAME", example))
    }

    fn arg_features(self) -> Self {
        self._arg(
            opt("features", "Space-separated list of features to activate").value_name("FEATURES"),
        )
        ._arg(opt("all-features", "Activate all available features"))
        ._arg(opt(
            "no-default-features",
            "Do not activate the `default` feature",
        ))
    }

    fn arg_release(self, release: &'static str) -> Self {
        self._arg(opt("release", release))
    }

    fn arg_doc(self, doc: &'static str) -> Self {
        self._arg(opt("doc", doc))
    }

    fn arg_target_triple(self, target: &'static str) -> Self {
        self._arg(opt("target", target).value_name("TRIPLE"))
    }

    fn arg_target_dir(self) -> Self {
        self._arg(
            opt("target-dir", "Directory for all generated artifacts").value_name("DIRECTORY"),
        )
    }

    fn arg_manifest_path(self) -> Self {
        self._arg(opt("manifest-path", "Path to Cargo.toml").value_name("PATH"))
    }

    fn arg_message_format(self) -> Self {
        self._arg(
            opt("message-format", "Error format")
                .value_name("FMT")
                .case_insensitive(true)
                .possible_values(&["human", "json", "short"])
                .default_value("human"),
        )
    }

    fn arg_build_plan(self) -> Self {
        self._arg(opt("build-plan", "Output the build plan in JSON"))
    }

    fn arg_new_opts(self) -> Self {
        self._arg(
            opt(
                "vcs",
                "\
                 Initialize a new repository for the given version \
                 control system (git, hg, pijul, or fossil) or do not \
                 initialize any version control at all (none), overriding \
                 a global configuration.",
            )
            .value_name("VCS")
            .possible_values(&["git", "hg", "pijul", "fossil", "none"]),
        )
        ._arg(opt("bin", "Use a binary (application) template [default]"))
        ._arg(opt("lib", "Use a library template"))
        ._arg(
            opt("edition", "Edition to set for the crate generated")
                .possible_values(&["2015", "2018"])
                .value_name("YEAR"),
        )
        ._arg(
            opt(
                "name",
                "Set the resulting package name, defaults to the directory name",
            )
            .value_name("NAME"),
        )
    }

    fn arg_index(self) -> Self {
        self._arg(opt("index", "Registry index to upload the package to").value_name("INDEX"))
            ._arg(
                opt("host", "DEPRECATED, renamed to '--index'")
                    .value_name("HOST")
                    .hidden(true),
            )
    }
}

impl AppExt for App<'static, 'static> {
    fn _arg(self, arg: Arg<'static, 'static>) -> Self {
        self.arg(arg)
    }
}

fn opt(name: &'static str, help: &'static str) -> Arg<'static, 'static> {
    Arg::with_name(name).long(name).help(help)
}

fn multi_opt(
    name: &'static str,
    value_name: &'static str,
    help: &'static str,
) -> Arg<'static, 'static> {
    // Note that all `.multiple(true)` arguments in Cargo should specify
    // `.number_of_values(1)` as well, so that `--foo val1 val2` is
    // **not** parsed as `foo` with values ["val1", "val2"].
    // `number_of_values` should become the default in clap 3.
    opt(name, help)
        .value_name(value_name)
        .multiple(true)
        .number_of_values(1)
}

pub trait ArgMatchesExt {
    fn value_of_u32(&self, name: &str) -> CargoResult<Option<u32>> {
        let arg = match self._value_of(name) {
            None => None,
            Some(arg) => Some(arg.parse::<u32>().map_err(|_| {
                clap::Error::value_validation_auto(format!("could not parse `{}` as a number", arg))
            })?),
        };
        Ok(arg)
    }

    /// Returns value of the `name` command-line argument as an absolute path
    fn value_of_path(&self, name: &str, config: &CargoConfig) -> Option<PathBuf> {
        self._value_of(name).map(|path| config.cwd().join(path))
    }

    fn jobs(&self) -> CargoResult<Option<u32>> {
        self.value_of_u32("jobs")
    }

    fn target(&self) -> Option<String> {
        self._value_of("target").map(|s| s.to_string())
    }

    fn manifest_path(&self, cargo_config: &CargoConfig) -> PathBuf {
        match self.value_of_path("manifest-path", cargo_config) {
            None => cargo_config.cwd().to_owned(),
            Some(path) => path.to_owned(),
        }
    }

    fn compile_options<'a>(
        &self,
        config: &'a CargoConfig,
        mode: CompileMode,
    ) -> CargoResult<CompileOptions<'a>> {
        let spec = Packages::from_flags(
            self._is_present("all"),
            self._values_of("exclude"),
            self._values_of("package"),
        )?;

        let message_format = match self._value_of("message-format") {
            None => MessageFormat::Human,
            Some(f) => {
                if f.eq_ignore_ascii_case("json") {
                    MessageFormat::Json
                } else if f.eq_ignore_ascii_case("human") {
                    MessageFormat::Human
                } else if f.eq_ignore_ascii_case("short") {
                    MessageFormat::Short
                } else {
                    panic!("Impossible message format: {:?}", f)
                }
            }
        };

        let mut build_config = BuildConfig::new(config, self.jobs()?, &self.target(), mode)?;
        build_config.message_format = message_format;
        build_config.release = self._is_present("release");
        build_config.build_plan = self._is_present("build-plan");
        if build_config.build_plan && !config.cli_unstable().unstable_options {
            Err(format_err!(
                "`--build-plan` flag is unstable, pass `-Z unstable-options` to enable it"
            ))?;
        };

        let opts = CompileOptions {
            config,
            build_config,
            features: self._values_of("features"),
            all_features: self._is_present("all-features"),
            no_default_features: self._is_present("no-default-features"),
            spec,
            filter: CompileFilter::new(
                self._is_present("lib"),
                self._values_of("bin"),
                self._is_present("bins"),
                self._values_of("test"),
                self._is_present("tests"),
                self._values_of("example"),
                self._is_present("examples"),
                self._values_of("bench"),
                self._is_present("benches"),
                self._is_present("all-targets"),
            ),
            target_rustdoc_args: None,
            target_rustc_args: None,
            local_rustdoc_args: None,
            export_dir: None,
        };
        Ok(opts)
    }

    fn _value_of(&self, name: &str) -> Option<&str>;

    fn _values_of(&self, name: &str) -> Vec<String>;

    fn _is_present(&self, name: &str) -> bool;
}

impl<'a> ArgMatchesExt for ArgMatches<'a> {
    fn _value_of(&self, name: &str) -> Option<&str> {
        self.value_of(name)
    }

    fn _values_of(&self, name: &str) -> Vec<String> {
        self.values_of(name)
            .unwrap_or_default()
            .map(|s| s.to_string())
            .collect()
    }

    fn _is_present(&self, name: &str) -> bool {
        self.is_present(name)
    }
}

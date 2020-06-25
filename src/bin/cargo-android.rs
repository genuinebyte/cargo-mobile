#![forbid(unsafe_code)]

use cargo_mobile::{
    android::{
        adb,
        config::{Config, Metadata},
        device::{Device, RunError, StacktraceError},
        env::{Env, Error as EnvError},
        target::{BuildError, CompileLibError, Target},
        NAME,
    },
    config::{
        metadata::{self, Metadata as OmniMetadata},
        Config as OmniConfig, LoadOrGenError,
    },
    define_device_prompt,
    device::PromptError,
    init, opts, os,
    target::{call_for_targets_with_fallback, TargetInvalid, TargetTrait as _},
    util::{
        cli::{self, Exec, GlobalFlags, Report, Reportable, TextWrapper},
        prompt,
    },
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = cli::bin_name(NAME), global_settings = cli::GLOBAL_SETTINGS, settings = cli::SETTINGS)]
pub struct Input {
    #[structopt(flatten)]
    flags: GlobalFlags,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(
        name = "init",
        about = "Creates a new project in the current working directory"
    )]
    Init {
        #[structopt(flatten)]
        please_destroy_my_files: cli::PleaseDestroyMyFiles,
        #[structopt(flatten)]
        reinstall_deps: cli::ReinstallDeps,
        #[structopt(
            long,
            help = "Open in Android Studio",
            parse(from_flag = opts::OpenIn::from_flag),
        )]
        open: opts::OpenIn,
    },
    #[structopt(name = "open", about = "Open project in Android Studio")]
    Open,
    #[structopt(name = "check", about = "Checks if code compiles for target(s)")]
    Check {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
    },
    #[structopt(name = "build", about = "Builds dynamic libraries for target(s)")]
    Build {
        #[structopt(name = "targets", default_value = Target::DEFAULT_KEY, possible_values = Target::name_list())]
        targets: Vec<String>,
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "run", about = "Deploys APK to connected device")]
    Run {
        #[structopt(flatten)]
        profile: cli::Profile,
    },
    #[structopt(name = "st", about = "Displays a detailed stacktrace for a device")]
    Stacktrace,
    #[structopt(name = "list", about = "Lists connected devices")]
    List,
}

#[derive(Debug)]
pub enum Error {
    EnvInitFailed(EnvError),
    DevicePromptFailed(PromptError<adb::device_list::Error>),
    TargetInvalid(TargetInvalid),
    ConfigFailed(LoadOrGenError),
    MetadataFailed(metadata::Error),
    InitFailed(init::Error),
    OpenFailed(bossy::Error),
    CheckFailed(CompileLibError),
    BuildFailed(BuildError),
    RunFailed(RunError),
    StacktraceFailed(StacktraceError),
    ListFailed(adb::device_list::Error),
}

impl Reportable for Error {
    fn report(&self) -> Report {
        match self {
            Self::EnvInitFailed(err) => err.report(),
            Self::DevicePromptFailed(err) => err.report(),
            Self::TargetInvalid(err) => Report::error("Specified target was invalid", err),
            Self::ConfigFailed(err) => err.report(),
            Self::MetadataFailed(err) => err.report(),
            Self::InitFailed(err) => err.report(),
            Self::OpenFailed(err) => Report::error("Failed to open project in Android Studio", err),
            Self::CheckFailed(err) => err.report(),
            Self::BuildFailed(err) => err.report(),
            Self::RunFailed(err) => err.report(),
            Self::StacktraceFailed(err) => err.report(),
            Self::ListFailed(err) => err.report(),
        }
    }
}

impl Exec for Input {
    type Report = Error;

    fn global_flags(&self) -> GlobalFlags {
        self.flags
    }

    fn exec(self, wrapper: &TextWrapper) -> Result<(), Self::Report> {
        define_device_prompt!(adb::device_list, adb::device_list::Error, Android);
        fn detect_target_ok<'a>(env: &Env) -> Option<&'a Target<'a>> {
            device_prompt(env).map(|device| device.target()).ok()
        }

        fn with_config(
            interactivity: opts::Interactivity,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config) -> Result<(), Error>,
        ) -> Result<(), Error> {
            let (config, _origin) = OmniConfig::load_or_gen(".", interactivity, wrapper)
                .map_err(Error::ConfigFailed)?;
            f(config.android())
        }

        fn with_config_and_metadata(
            interactivity: opts::Interactivity,
            wrapper: &TextWrapper,
            f: impl FnOnce(&Config, &Metadata) -> Result<(), Error>,
        ) -> Result<(), Error> {
            with_config(interactivity, wrapper, |config| {
                let metadata =
                    OmniMetadata::load(&config.app().root_dir()).map_err(Error::MetadataFailed)?;
                f(config, &metadata.android)
            })
        }

        fn open_in_android_studio(config: &Config) -> Result<(), Error> {
            os::open_file_with("Android Studio", config.project_dir()).map_err(Error::OpenFailed)
        }

        let Self {
            flags:
                GlobalFlags {
                    noise_level,
                    interactivity,
                },
            command,
        } = self;
        let env = Env::new().map_err(Error::EnvInitFailed)?;
        match command {
            Command::Init {
                please_destroy_my_files:
                    cli::PleaseDestroyMyFiles {
                        please_destroy_my_files,
                    },
                reinstall_deps: cli::ReinstallDeps { reinstall_deps },
                open,
            } => {
                let config = init::exec(
                    wrapper,
                    interactivity,
                    please_destroy_my_files,
                    reinstall_deps,
                    opts::OpenIn::Nothing,
                    Some(vec!["android".into()]),
                    None,
                    ".",
                )
                .map_err(Error::InitFailed)?;
                if open.editor() {
                    open_in_android_studio(config.android())
                } else {
                    Ok(())
                }
            }
            Command::Open => with_config(interactivity, wrapper, open_in_android_studio),
            Command::Check { targets } => {
                with_config_and_metadata(interactivity, wrapper, |config, metadata| {
                    call_for_targets_with_fallback(
                        targets.iter(),
                        &detect_target_ok,
                        &env,
                        |target: &Target| {
                            target
                                .check(config, metadata, &env, noise_level, interactivity)
                                .map_err(Error::CheckFailed)
                        },
                    )
                    .map_err(Error::TargetInvalid)?
                })
            }
            Command::Build {
                targets,
                profile: cli::Profile { profile },
            } => with_config_and_metadata(interactivity, wrapper, |config, metadata| {
                call_for_targets_with_fallback(
                    targets.iter(),
                    &detect_target_ok,
                    &env,
                    |target: &Target| {
                        target
                            .build(config, metadata, &env, noise_level, interactivity, profile)
                            .map_err(Error::BuildFailed)
                    },
                )
                .map_err(Error::TargetInvalid)?
            }),
            Command::Run {
                profile: cli::Profile { profile },
            } => with_config(interactivity, wrapper, |config| {
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .run(config, &env, noise_level, profile)
                    .map_err(Error::RunFailed)
            }),
            Command::Stacktrace => with_config(interactivity, wrapper, |config| {
                device_prompt(&env)
                    .map_err(Error::DevicePromptFailed)?
                    .stacktrace(config, &env)
                    .map_err(Error::StacktraceFailed)
            }),
            Command::List => adb::device_list(&env)
                .map_err(Error::ListFailed)
                .map(|device_list| {
                    prompt::list_display_only(device_list.iter(), device_list.len());
                }),
        }
    }
}

fn main() {
    cli::exec::<Input>(NAME)
}

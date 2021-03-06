use crate::util::cli::{Report, Reportable};
use std::{
    fmt::{self, Display},
    fs::File,
    io,
    num::ParseIntError,
    path::{Path, PathBuf},
};

const MIN_NDK_VERSION: Version = Version {
    major: 19,
    minor: 0,
};

#[cfg(target_os = "macos")]
pub fn host_tag() -> &'static str {
    "darwin-x86_64"
}

#[cfg(target_os = "linux")]
pub fn host_tag() -> &'static str {
    "linux-x86_64"
}

#[cfg(all(target_os = "window", target_pointer_width = "32"))]
pub fn host_tag() -> &'static str {
    "windows"
}

#[cfg(all(target_os = "window", target_pointer_width = "64"))]
pub fn host_tag() -> &'static str {
    "windows-x86_64"
}

#[derive(Clone, Copy, Debug)]
pub enum Compiler {
    Clang,
    Clangxx,
}

impl Compiler {
    fn as_str(&self) -> &'static str {
        match self {
            Compiler::Clang => "clang",
            Compiler::Clangxx => "clang++",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Binutil {
    Ar,
    #[allow(dead_code)]
    Ld,
}

impl Binutil {
    fn as_str(&self) -> &'static str {
        match self {
            Binutil::Ar => "ar",
            Binutil::Ld => "ld",
        }
    }
}

#[derive(Debug)]
pub struct MissingToolError {
    name: &'static str,
    tried_path: PathBuf,
}

impl Display for MissingToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Missing tool `{}`; tried at {:?}.",
            self.name, self.tried_path
        )
    }
}

#[derive(Debug)]
pub enum VersionError {
    OpenFailed {
        path: PathBuf,
        cause: io::Error,
    },
    ParseFailed {
        path: PathBuf,
        cause: java_properties::PropertiesError,
    },
    VersionMissing {
        path: PathBuf,
    },
    ComponentNotNumerical {
        path: PathBuf,
        component: String,
        cause: ParseIntError,
    },
    TooFewComponents {
        path: PathBuf,
        version: String,
    },
}

impl Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenFailed { path, cause } => {
                write!(f, "Failed to open {:?}: {}", path, cause)
            }
            Self::ParseFailed { path, cause } => {
                write!(f, "Failed to parse {:?}: {}", path, cause)
            }
            Self::VersionMissing { path } =>{
                write!(f, "No version number was present in {:?}.", path)
            }
            Self::ComponentNotNumerical { path, component, cause } => write!(
                f,
                "Properties at {:?} contained a version component {:?} that wasn't a valid number: {}",
                path, component, cause
            ),
            Self::TooFewComponents { path, version } => write!(
                f,
                "Version {:?} in properties file {:?} didn't have as many components as expected.",
                path, version
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version {
    major: u32,
    minor: u32,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.major)?;
        if self.minor != 0 {
            write!(
                f,
                "{}",
                (b'a'..=b'z')
                    .map(char::from)
                    .nth(self.minor as _)
                    .expect("NDK minor version exceeded the number of letters in the alphabet")
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    // TODO: link to docs/etc.
    NdkHomeNotSet(std::env::VarError),
    NdkHomeNotADir,
    VersionLookupFailed(VersionError),
    VersionTooLow {
        you_have: Version,
        you_need: Version,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NdkHomeNotSet(err) => write!(
                f,
                "Have you installed the NDK? The `NDK_HOME` environment variable isn't set, and is required: {}",
                err,
            ),
            Self::NdkHomeNotADir => write!(
                f,
                "Have you installed the NDK? The `NDK_HOME` environment variable is set, but doesn't point to an existing directory."
            ),
            Self::VersionLookupFailed(err) => {
                write!(f, "Failed to lookup version of installed NDK: {}", err)
            }
            Self::VersionTooLow { you_have, you_need } => write!(
                f,
                "At least NDK {} is required (you currently have NDK {})",
                you_need,
                you_have,
            ),
        }
    }
}

impl Reportable for Error {
    fn report(&self) -> Report {
        Report::error("Failed to initialize NDK environment", self)
    }
}

#[derive(Debug)]
pub struct Env {
    ndk_home: PathBuf,
}

impl Env {
    pub fn new() -> Result<Self, Error> {
        let ndk_home = std::env::var("NDK_HOME")
            .map_err(Error::NdkHomeNotSet)
            .map(PathBuf::from)
            .and_then(|ndk_home| {
                if ndk_home.is_dir() {
                    Ok(ndk_home)
                } else {
                    Err(Error::NdkHomeNotADir)
                }
            })?;
        let env = Self { ndk_home };
        let version = env.version().map_err(Error::VersionLookupFailed)?;
        if version >= MIN_NDK_VERSION {
            Ok(env)
        } else {
            Err(Error::VersionTooLow {
                you_have: version,
                you_need: MIN_NDK_VERSION,
            })
        }
    }

    pub fn home(&self) -> &Path {
        &self.ndk_home
    }

    pub fn version(&self) -> Result<Version, VersionError> {
        let path = self.ndk_home.join("source.properties");
        let file = File::open(&path).map_err(|cause| VersionError::OpenFailed {
            path: path.clone(),
            cause,
        })?;
        let props = java_properties::read(file).map_err(|cause| VersionError::ParseFailed {
            path: path.clone(),
            cause,
        })?;
        let revision = props
            .get("Pkg.Revision")
            .ok_or_else(|| VersionError::VersionMissing { path: path.clone() })?;
        // The possible revision formats can be found in the comments of
        // `$NDK_HOME/build/cmake/android.toolchain.cmake` - only the last component
        // can be non-numerical, which we're not using anyway. If that changes,
        // then the aforementioned file contains a regex we can use.
        let components = revision
            .split('.')
            .take(2)
            .map(|component| {
                component
                    .parse::<u32>()
                    .map_err(|cause| VersionError::ComponentNotNumerical {
                        path: path.clone(),
                        component: component.to_owned(),
                        cause,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if components.len() == 2 {
            Ok(Version {
                major: components[0],
                minor: components[1],
            })
        } else {
            Err(VersionError::TooFewComponents {
                path,
                version: revision.to_owned(),
            })
        }
    }

    pub fn tool_dir(&self) -> Result<PathBuf, MissingToolError> {
        let path = self
            .ndk_home
            .join(format!("toolchains/llvm/prebuilt/{}/bin", host_tag()));
        if path.is_dir() {
            Ok(path)
        } else {
            // TODO: this might be too silly
            Err(MissingToolError {
                name: "literally all of them",
                tried_path: path,
            })
        }
    }

    pub fn compiler_path(
        &self,
        compiler: Compiler,
        triple: &str,
        min_api: u32,
    ) -> Result<PathBuf, MissingToolError> {
        let path = self
            .tool_dir()?
            .join(format!("{}{}-{}", triple, min_api, compiler.as_str()));
        if path.is_file() {
            Ok(path)
        } else {
            Err(MissingToolError {
                name: compiler.as_str(),
                tried_path: path,
            })
        }
    }

    pub fn binutil_path(
        &self,
        binutil: Binutil,
        triple: &str,
    ) -> Result<PathBuf, MissingToolError> {
        let path = self
            .tool_dir()?
            .join(format!("{}-{}", triple, binutil.as_str()));
        if path.is_file() {
            Ok(path)
        } else {
            Err(MissingToolError {
                name: binutil.as_str(),
                tried_path: path,
            })
        }
    }
}

/// PEP 440 package version, derived from the Cargo package version by `build.rs`.
pub const VERSION: &str = env!("CFABRIC_PEP440_VERSION");
#[allow(non_upper_case_globals)]
pub const __version__: &str = VERSION;
pub const NAME: &str = "Context-Fabric";
pub const BANNER: &str = concat!("This is Context-Fabric ", env!("CFABRIC_PEP440_VERSION"));

pub const OTYPE: &str = "otype";
pub const OSLOTS: &str = "oslots";
pub const OTEXT: &str = "otext";

pub const SEARCH_FAIL_FACTOR: u32 = 4;

/// The text format `T.text` uses when none is given (TF `DEFAULT_FORMAT`).
pub const DEFAULT_FORMAT: &str = "text-orig-full";

pub const CFR_VERSION: &str = "4";

/// PEP 440 package version, derived from the Cargo package version by `build.rs`.
pub const VERSION: &str = env!("CFABRIC_PEP440_VERSION");
#[allow(non_upper_case_globals)]
pub const __version__: &str = VERSION;
pub const NAME: &str = "Context-Fabric";
pub const BANNER: &str = concat!("This is Context-Fabric ", env!("CFABRIC_PEP440_VERSION"));
pub const API_VERSION: u32 = 3;

pub const OTYPE: &str = "otype";
pub const OSLOTS: &str = "oslots";
pub const OTEXT: &str = "otext";
pub const OVOLUME: &str = "ovolume";
pub const OWORK: &str = "owork";
pub const OINTERF: &str = "ointerfrom";
pub const OINTERT: &str = "ointerto";
pub const OMAP: &str = "omap";
pub const WARP: &[&str] = &[OTYPE, OSLOTS, OTEXT];

pub const ORG: &str = "codykingham";
pub const REPO: &str = "context-fabric";
pub const RELATIVE: &str = "tf";

pub const GH: &str = "github";
pub const GL: &str = "gitlab";
pub const URL_GH: &str = "https://github.com";
pub const URL_GH_API: &str = "https://api.github.com";
pub const URL_GH_UPLOAD: &str = "https://uploads.github.com";
pub const URL_GL: &str = "https://gitlab.com";
pub const URL_GL_API: &str = "https://api.gitlab.com";
pub const URL_GL_UPLOAD: &str = "https://uploads.gitlab.com";
pub const URL_NB: &str = "https://nbviewer.jupyter.org";
pub const URL_CF_DOCS: &str = "https://github.com/Context-Fabric/context-fabric";

pub const PROTOCOL: &str = "http://";
pub const HOST: &str = "localhost";
pub const PORT_BASE: u32 = 10000;

pub const DOI_DEFAULT: &str = "no DOI";
pub const DOI_URL_PREFIX: &str = "https://doi.org";
pub const BRANCH_DEFAULT: &str = "master";
pub const BRANCH_DEFAULT_NEW: &str = "main";

pub const YARN_RATIO: f64 = 1.25;
pub const TRY_LIMIT_FROM: i64 = 40;
pub const TRY_LIMIT_TO: i64 = 40;
pub const SEARCH_FAIL_FACTOR: u32 = 4;

/// The text format `T.text` uses when none is given (TF `DEFAULT_FORMAT`).
pub const DEFAULT_FORMAT: &str = "text-orig-full";

pub const CFM_VERSION: &str = "2";
pub const CFR_VERSION: &str = "4";
pub const NODE_DTYPE: &str = "uint32";
pub const RANK_DTYPE: &str = "uint32";
pub const INDEX_DTYPE: &str = "uint32";
pub const TYPE_DTYPE: &str = "uint8";
pub const MISSING_INT: i64 = -1;
pub const CONFIG_MISSING_STR_INDEX: u32 = 0xFFFF_FFFF;

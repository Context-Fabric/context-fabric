//! Derives the Python (PEP 440) package version from the Cargo package version.
//!
//! `Cargo.toml` is the single source of truth for the release version. Maturin
//! publishes the wheel under the PEP 440 rendering of that version (the
//! `pyproject.toml` declares `version` as dynamic), and this build script exports
//! the same rendering as `CFABRIC_PEP440_VERSION` so `cfabric.VERSION`,
//! `cfabric.__version__`, and the banner can never drift from the wheel metadata.

use std::env;

/// Render a SemVer version (`MAJOR.MINOR.PATCH[-PRE]`) the way maturin does for
/// the wheel metadata: `-alpha.N` -> `aN`, `-beta.N` -> `bN`, `-rc.N` -> `rcN`.
fn pep440(version: &str) -> String {
    let Some((release, pre)) = version.split_once('-') else {
        return version.to_string();
    };
    let (label, number) = pre.split_once('.').unwrap_or((pre, "0"));
    let marker = match label {
        "alpha" | "a" => "a",
        "beta" | "b" => "b",
        "rc" => "rc",
        other => panic!(
            "unsupported pre-release label {other:?} in Cargo version {version:?}; \
             use alpha.N, beta.N, or rc.N"
        ),
    };
    assert!(
        number.chars().all(|ch| ch.is_ascii_digit()) && !number.is_empty(),
        "pre-release number in Cargo version {version:?} must be numeric"
    );
    format!("{release}{marker}{number}")
}

fn main() {
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo sets CARGO_PKG_VERSION");
    println!(
        "cargo:rustc-env=CFABRIC_PEP440_VERSION={}",
        pep440(&version)
    );
    println!("cargo:rerun-if-changed=Cargo.toml");
}

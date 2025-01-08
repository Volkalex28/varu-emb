#![feature(iterator_try_collect)]

use proc_macro::TokenStream as TS;
use proc_macro2::Span;
use quote::ToTokens;
use std::{env, path};
use syn::parse::Parse;
use syn::{parse, Error, Result};

type Crates = toml::Table;

mod cfg;

fn implementation<T: Parse + ToTokens>(input: TS) -> TS {
    let output = match parse::<T>(input) {
        Ok(data) => data.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

#[proc_macro_derive(Config, attributes(varuemb_cfg))]
pub fn cfg(input: TS) -> TS {
    implementation::<cfg::Cfg>(input)
}

/// Attempts to find the root path of the project.
///
/// This function tries to locate the root directory of the project by first checking
/// the `OUT_DIR` environment variable. If that's not available, it searches through
/// the command-line arguments for the path to the `varuemb` library. Once a path is found,
/// it's cleaned up by trimming trailing directories until the "target" directory is reached,
/// and then moving one level up from there.
///
/// # Returns
///
/// Returns `Some(PathBuf)` containing the root path of the project if successful,
/// or `None` if the root path couldn't be determined.
fn find_root_path() -> Option<path::PathBuf> {
    let mut args = env::args();

    let mut path = env::var("OUT_DIR").ok();
    if path.is_none() {
        // Try to find the path to the varuemb rlib
        while let Some(arg) = (&mut args).skip_while(|arg| *arg != "--extern").nth(1) {
            if arg.starts_with("varuemb") {
                path = Some(arg)
            }
        }
    }

    // Clean up the path via trimming trailing directories until "target"
    let mut path = path::PathBuf::from(path.as_ref()?.trim_start_matches("varuemb="));
    while !path.ends_with("target") {
        if !path.pop() {
            return None;
        }
    }

    path.pop();

    Some(path)
}

/// Loads the configuration for the current package from a TOML file.
///
/// This function attempts to locate the root directory of the workspace,
/// reads the TOML configuration file, and extracts the configuration
/// specific to the current package.
///
/// # Returns
///
/// A `Result` containing a `toml::Table` with the configuration for the
/// current package if successful, or an `Error` if the configuration
/// cannot be loaded or parsed.
fn load() -> Result<(Option<toml::Table>, path::PathBuf)> {
    let pkg_name = env::var("CARGO_PKG_NAME").map_err(|err| Error::new(Span::call_site(), err))?;

    let Some(mut path) = find_root_path() else {
        return if env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE").is_ok() {
            Ok((None, path::PathBuf::new()))
        } else {
            // todo!("Args: {:#?}/nVars: {:#?}", std::env::args(), std::env::vars());
            Err(Error::new(Span::call_site(), "Unable to locate root directory"))
        };
    };
    path.push("cfg.toml");

    let contents = std::fs::read_to_string(&path).map_err(|err| Error::new(Span::call_site(), err))?;
    let mut configs = toml::from_str::<Crates>(&contents).map_err(|err| Error::new(Span::call_site(), err))?;

    let Some(value) = configs.remove(pkg_name.as_str()) else {
        return Ok((None, path));
    };

    if let toml::Value::Table(config) = value {
        Ok((Some(config), path))
    } else {
        Err(Error::new(Span::call_site(), format!("Config for package '{}' is not toml table", pkg_name)))
    }
}

mod tokens {
    syn::custom_keyword!(varuemb);
    syn::custom_keyword!(cfg);
}

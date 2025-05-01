use anyhow::Context;
use std::env;
use std::path::PathBuf;

pub fn home() -> anyhow::Result<PathBuf> {
    env::var_os("HOME")
        .context("HOME is not set")
        .map(PathBuf::from)
}

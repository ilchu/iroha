//! Provides macros used to get the version

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    vergen::EmitBuilder::builder()
        .git_sha(true)
        .cargo_target_triple()
        .emit()?;
    Ok(())
}

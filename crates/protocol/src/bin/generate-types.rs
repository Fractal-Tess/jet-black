use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate-types <output-path>")?;
    let declarations = protocol::typescript_declarations();
    if matches!(fs::read_to_string(&output), Ok(existing) if existing == declarations) {
        return Ok(());
    }
    fs::write(output, declarations)?;
    Ok(())
}

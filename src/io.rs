use std::{
    fs::File,
    io::{Read, Write},
};

use ropey::Rope;

/// Save the content of the rope to the specified filepath
pub fn save(rope: &Rope, filepath: &str) -> std::io::Result<()> {
    let mut file = File::create(filepath)?;

    file.write_all(rope.to_string().as_bytes());

    Ok(())
}

/// Read the file at filepath and return a rope
pub fn load(filepath: &str) -> std::io::Result<Rope> {
    let mut file = File::create(filepath)?;

    let mut buffer_string = String::new();
    file.read_to_string(&mut buffer_string)?;

    Ok(Rope::from_str(&buffer_string))
}

use std::{
    fs::File,
    io::{Read, Write},
};

use crate::buffer::Buffer;
use ropey::Rope;

/// Save the content of the rope to the specified filepath
pub fn save(buffer: &mut Buffer, filepath: Option<String>) -> Result<(), String> {
    let filepath = filepath.or(buffer.filepath.clone());

    println!("filepath: {:?}", filepath);

    if let Some(filepath) = filepath {
        if let Ok(mut file) = File::create(filepath) {
            if let Err(e) = file.write_all(buffer.text.to_string().as_bytes()) {
                return Err(format!("Could not write to file: {}", e.to_string()));
            };
            return Ok(());
        };
        return Err("Could not open file".to_owned());
    }

    Err("No filepath specified".to_owned())
}

/// Read the file at filepath and return a rope
pub fn load(filepath: &str) -> std::io::Result<Rope> {
    let mut file = File::open(filepath)?;
    println!("{}", filepath);

    let mut buffer_string = String::new();
    file.read_to_string(&mut buffer_string)?;

    println!("{}", buffer_string);

    Ok(Rope::from_str(&buffer_string))
}

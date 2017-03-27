use std::fs::File;
use std::io::Read;
use std::process::exit;

pub fn read_binary(path: &str) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();

    let mut f = match File::open(path) {
        Ok(x) => x,
        Err(why) => {
            // XXX return error to caller.. how?!11 so they can call .except() ..=?!?1
            println!("Could not open file {}: {}", path, why);
            exit(1);
        }
    };

    match f.read_to_end(&mut buffer) {
        Ok(x) => x,
        Err(why) => {
            println!("could not read contents of file: {}", why);
            exit(1);
        }
    };

    buffer
}


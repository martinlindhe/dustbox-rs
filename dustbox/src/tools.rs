use std::fs::File;
use std::io::Read;
use std::io::Error;

pub fn read_binary(path: &str) -> Result<Vec<u8>, Error> {
    let mut buffer: Vec<u8> = Vec::new();

    let mut f = match File::open(path) {
        Ok(x) => x,
        Err(why) => return Err(why),
    };

    match f.read_to_end(&mut buffer) {
        Ok(_) => Ok(buffer),
        Err(why) => Err(why),
    }
}

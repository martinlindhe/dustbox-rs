use clap::{Arg, App};

use dustbox::tools::read_binary;
use dustbox::format::ExeFile;

const VERSION: &str = "0.1";

fn main() {
    let matches = App::new("dustbox-exeinfo")
            .version(VERSION)
            .arg(Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1))
            .get_matches();

    let filename = matches.value_of("INPUT").unwrap();
    println!("dustbox-exeinfo {} - {}", VERSION, filename);

    let data = match read_binary(filename) {
        Ok(data) => data,
        Err(e) => panic!(e),
    };

    match ExeFile::from_data(&data) {
        Ok(exe) => exe.print_details(),
        Err(e) => panic!(e),
    }
}

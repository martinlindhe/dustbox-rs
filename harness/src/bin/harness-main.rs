use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

extern crate clap;
use clap::{Arg, App};

use colored::*;
use tera::{Tera, Context};
use serde::{Serialize, Deserialize};

use dustbox::machine::Machine;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SetDocument {
    name: String,
    default_instructions: usize,
    root: String,

    /// filename or filename,instructions
    set: Vec<String>,
}

fn main() {
    let matches = App::new("dustbox-harness")
        .version("0.1")
        .arg(Arg::with_name("INPUT")
            .help("Sets the test harness rom set file to use")
            .required(true)
            .index(1))
        .get_matches();

    let filename = matches.value_of("INPUT").unwrap();

    let data = fs::read_to_string(filename).expect("Unable to read file");
    let set: SetDocument = serde_yaml::from_str(&data).unwrap();

    run_and_save_video_frames(&set);
}

fn run_and_save_video_frames(doc: &SetDocument) {

    let mut out_images = vec![];

    for bin in &doc.set {
        // if comma, split in path & instructions
        let parts: Vec<&str> = bin.split(",").collect();

        let bin = parts[0];

        let mut execute_instrs = doc.default_instructions;
        if parts.len() > 1 {
            execute_instrs = parts[1].parse::<usize>().unwrap();
        }
    
        println!("{}: {}", doc.name.white(), bin.yellow());

        let mut machine = Machine::deterministic();
        let bin_path = format!("{}{}", doc.root, bin);

        if let Some(e) = machine.load_executable_file(&bin_path, 0x0329) {
            panic!("error {}: {}", e, bin_path);
        };

        machine.execute_instructions(execute_instrs);

        if !Path::new(&format!("docs/render/{}", doc.name)).exists() {
            if let Err(e) = fs::create_dir(&format!("docs/render/{}", doc.name)) {
                panic!("create_dir failed {}", e);
            }
        }

        let rel_path = Path::new(&bin);
        let stem = rel_path.file_stem().unwrap_or_else(|| OsStr::new(""));
        let mut filename = OsString::new(); // XXX base on dirname
        let outname = &format!("render/{}/{:02x}_", doc.name, machine.gpu_mut().mode.mode);
        filename.push(format!("docs/{}", outname));
        filename.push(stem.to_os_string());
        filename.push(".png");

        if write_video_frame_to_disk(&mut machine, filename.to_str().unwrap()) {
            let mut pub_filename = String::new();
            pub_filename.push_str(&outname);
            pub_filename.push_str(stem.to_str().unwrap());
            pub_filename.push_str(".png");
            out_images.push(pub_filename);
        } else {
            println!("failed to write {} to disk", filename.to_str().unwrap());
        }
    }

    let mut tera = match Tera::new("harness/templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // disable auto-escaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    out_images.sort();
    context.insert("out_images", &out_images);
    // add stuff to context
    match tera.render("test_category.tpl.html", &context) {
        Ok(res) => {
            let mut f = File::create(format!("docs/{}.html", doc.name)).expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => panic!(format!("{}", why)),
    }
}

// returns true on success
fn write_video_frame_to_disk(machine: &mut Machine, pngfile: &str) -> bool {
    let frame = machine.gpu().render_frame(&machine.mmu);
    if frame.data.is_empty() {
        println!("ERROR: no frame rendered");
        return false;
    }
    let img = frame.draw_image();
    if let Err(why) = img.save(pngfile) {
        println!("save err: {:?}", why);
        return false;
    }
    true
}

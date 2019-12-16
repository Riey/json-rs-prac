use std::env::args;
use std::path::Path;

fn main() {
    if let Some(file) = args().skip(1).filter(|s| Path::new(s).exists()).next() {
        let content = std::fs::read_to_string(file).unwrap();

        let value = json_rs_prac::value(&content).unwrap().1;

        println!("{:#?}", value);
    } else {
        println!("Usage json-rs-prac [file path]");
    }
}

use crate::parser::ScrobbleLog;

mod parser;

fn main() {
    let filename = std::env::args().nth(1).expect("expected 1 argument");
    let log = ScrobbleLog::parse_file(filename);
    println!("{:?}", log.unwrap());
}

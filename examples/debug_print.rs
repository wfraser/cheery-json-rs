extern crate cheery_json;

fn main() {
    // Parse stdin as a JSON object and dump it to stdout.
    println!("{:?}", cheery_json::parse(std::io::stdin()));
}


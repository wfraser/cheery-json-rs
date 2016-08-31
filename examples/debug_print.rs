extern crate cheery_json;

fn main() {
    println!("{:?}", cheery_json::parse(std::io::stdin()));
}


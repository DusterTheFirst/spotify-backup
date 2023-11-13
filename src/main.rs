use serde::{Deserialize, Serialize};

struct Secrets {

}

#[derive(Serialize, Deserialize)]
struct Backup {
    github: String,
    repository: String,
    spotify: String,
}

fn main() {
    println!("Hello, world!");
}

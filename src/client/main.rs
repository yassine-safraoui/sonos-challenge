use std::thread::sleep;
use std::time::Duration;

fn main() {
    println!("Hello world from client!");
    loop {
        sleep(Duration::from_secs(1));
        println!("Hi again!");
    }
}

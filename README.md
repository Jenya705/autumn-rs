# Autumn

Autumn is a framework for rust to develop applications based on beans (aka services) structure easily

## Usage

`main.rs`

```rust
#[autumn::mod]
mod main {
    #[autumn::service(initialize = new)]
    pub struct CountService {
        counter: std::sync::Mutex<i32>,
    }

    impl CountService {
        pub fn new() -> Self {
            Self {
                counter: std::sync::Mutex::new(0),
            }
        }

        pub fn increment(&self) -> i32 {
            let mut value = self.counter.lock().unwrap();
            let ret = value;
            *value += 1;
            ret
        }
    }
    
    #[autumn::schedule(interval = "500ms")]
    pub async fn run(count_service: &CountService) {
        println!("{}", count_service.increment())
    }
    
}

pub use main::*;

#[autumn::main]
fn main() {}

```

use std::fs;
use std::time::SystemTime;

fn main() {
    println!("Hello from KrakeOS WASM!");

    // Test 1: Time
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => println!("Current WASI Time (nanos): {}", n.as_nanos()),
        Err(_) => println!("Time error!"),
    }

    // Test 2: Filesystem (Read)
    println!("Reading /sys/bin directory...");
    if let Ok(entries) = fs::read_dir("/") {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("Found: {:?}", entry.path());
            }
        }
    } else {
        println!("Failed to read directory.");
    }

    // Test 3: Filesystem (Write)
    let test_file = "/wasm_hello.txt";
    println!("Writing to {}...", test_file);
    if fs::write(test_file, "WASI works on KrakeOS!").is_ok() {
        println!("Write successful.");
        
        if let Ok(content) = fs::read_to_string(test_file) {
            println!("Read back content: '{}'", content);
        }
    } else {
        println!("Write failed.");
    }

    println!("WASM Test Complete.");
}

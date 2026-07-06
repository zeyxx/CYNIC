use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    println!("CYNIC Jarvis Sensor (Windows Endpoint) starting...");
    // TODO: Implement rdev key/mouse hooks
    // TODO: Implement windows API active window tracking
    // TODO: Send loop to CYNIC_REST_ADDR / Jarvis Brain
    
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        // Mock payload emit
        // println!("Emitting cognitive-signal-v1...");
    }
}

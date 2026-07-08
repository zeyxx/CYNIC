use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    println!("CYNIC Cognitive Sensor (host endpoint) starting...");
    // TODO: Implement rdev key/mouse hooks
    // TODO: Implement windows API active window tracking
    // TODO: Send loop to CYNIC_REST_ADDR / cognitive signal ingestion
    
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        // Mock payload emit
        // println!("Emitting cognitive-signal-v1...");
    }
}

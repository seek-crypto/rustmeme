use rand::Rng;
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;
use std::thread;
use std::time::{Duration, SystemTime};

pub fn start_random_timer(tx: UnboundedSender<String>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut rng = rand::rng();
        
        let mut virtual_token_reserves =  100_000_000_000i64;
        let mut virtual_sol_reserves =  100_000_000i64;
        let mut slot = 1000;
        loop {                                    
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            // create pumpfun bonding curve account modify event
            let message = json!({
                "slot": slot, // simulate solana slot 
                "virtual_token_reserves" : virtual_token_reserves, //simulate pumpfun virtual_token_reserves
                "virtual_sol_reserves" : virtual_sol_reserves, //simulate pumpfun virtual_sol_reserves
                "timestamp": timestamp
            }).to_string();
            
            // send pumpfun bonding curve event to main 
            if let Err(e) = tx.send(message) {
                eprintln!("Error sending message: {}", e);
                break;
            }

            // Simulate price fluctuation
            virtual_token_reserves += rng.random_range(-10_000..=10_000);
            virtual_sol_reserves += rng.random_range(-10_000..=10_000);
            
            // Simlulate solana block slot++ 
            slot += 1;

            // sleep some time 
            let random_ms: u64 = rng.random_range(100..=1_000);
            thread::sleep(Duration::from_millis(random_ms));
        }
    })
}

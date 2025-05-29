pub mod mock_source;
pub mod kline;
pub mod websocket;
use std::error::Error;

use actix_web::{web, App, HttpServer, middleware::Logger, HttpResponse, Result};
use kline::{KLineAggregator, KLineData};
use tokio::task;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;


// Handler for serving the static HTML file
async fn serve_index() -> Result<HttpResponse> {
    let html_content = include_str!("../websocket_client.html");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html_content))
}

// Producer: Send messages to Kafka
fn produce_pumpfun_messages(producer: &FutureProducer, kline : &KLineData) -> Result<(), Box<dyn Error>>  {
    // Clone the data we need for the async task
    
    let producer_clone = producer.clone();

    let message = serde_json::to_string(&kline)?; 

    let time_window_seconds = kline.time_window_seconds;
    // send kline data to kafka (fire and forget)    
    task::spawn(async move {
        let key = format!("key_{}", time_window_seconds );            
        let record = FutureRecord::to("demo")
            .key(&key)
            .payload(&message);
            
        match producer_clone.send(record, Timeout::Never).await {
            Ok((_partition,_offsett)) => {
                //println!("send kafka msg ok: partition={}, offset={}", partition, offset);
            }
            Err((e, _)) => {
                eprintln!("send kafka msg error: {}", e);
            }
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    
    // Start message processing task
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = mock_source::start_random_timer(tx);
       
    // Spawn message processing task    
    task::spawn(async move {
        //let mut message_count = 0;
        let mut aggregator  = KLineAggregator::new();
                
        let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .create().unwrap();

        // Process messages from channel
        while let Some(msg) = rx.recv().await {
            match serde_json::from_str::<MockPumpfunMessage>(&msg) {
                Ok(data) => {
                    let price = data.price();
                    let timestamp = data.timestamp;
                    
                    // Add price data to k-line aggregator                    
                    aggregator.update(price, timestamp);
                    aggregator.send_to_kafka_for_each(produce_pumpfun_messages, &producer);
                        
                    //message_count += 1;
                    
                    // println!("Received: pumpfun price {:.12} (slot: {}, timestamp: {})", 
                    //             price, data.slot, timestamp);
                    
                    // Print current k-line status every 10 messages
                    // if message_count % 10 == 0 {
                    //     aggregator.print_current_status();
                    // }
                },
                Err(e) => eprintln!("Failed to parse JSON: {}", e),
            }
        }
    });
    
    // Start websocket server
    println!("Starting WebSocket server on http://127.0.0.1:8080");    

    HttpServer::new(move || {
        App::new()            
            .wrap(Logger::default())
            .route("/", web::get().to(serve_index))
            .route("/ws", web::get().to(websocket::websocket_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct MockPumpfunMessage {
    slot: i32,
    virtual_token_reserves: i64,
    virtual_sol_reserves: i64,
    timestamp : u64
}

impl MockPumpfunMessage {
    fn price(&self) -> f64 {
        self.virtual_sol_reserves as f64 / self.virtual_token_reserves as f64
    }
}

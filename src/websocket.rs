use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use futures_util::StreamExt;
use crate::kline::KLineData;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message as KafkaMessage;


struct WebSocketSession {
    pub session: Session,    
    pub subscriptions: Option<String>
}

impl WebSocketSession {
    fn new(session: Session) -> Self {
        Self {
            session,            
            subscriptions: None
        }
    }

    async fn text(&mut self, msg: &str) {
        let _ = self.session.text(msg).await;
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Text(text) => {
                let text = text.trim();
                
                // Handle simple subscription commands
                if text.starts_with("sub-") {
                    let window_str = &text[4..]; // Remove "sub-" prefix
                    match Self::parse_time_window_to_kafka_msg_key(window_str) {
                        Ok(time_window) => {
                            self.subscriptions = Some(time_window.to_string());
                            println!("Client subscribed to {} via simple command", window_str);
                            let _ = self.session.text(format!("Subscribed to {}", window_str)).await;
                        }
                        Err(err) => {                            
                            let _ = self.session.text(format!("Error: {}", err)).await;
                        }
                    }
                    return;
                }

                // Handle global unsubscribe command
                if text == "unsub" {
                    self.subscriptions = None;
                    println!("Client unsubscribed from all channels");
                    let _ = self.session.text("Unsubscribed from all channels").await;
                    return;
                }
            }            
            Message::Close(_) => {
                println!("WebSocket client disconnected");
            }
            _ => {}
        }
    }
    

    fn parse_time_window_to_kafka_msg_key(window_str: &str) -> Result<&str, actix_web::Error> {
        match window_str {
            "1s" => Ok("key_1"),
            "1m" => Ok("key_60"),
            "5m" => Ok("key_300"),
            "1h" => Ok("key_3600"),
            _ => Err(actix_web::error::ErrorBadRequest(format!("Invalid time window: {}", window_str))),
        }
    }

}

pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload    
) -> Result<HttpResponse, Error> {
    let (response, session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    
    println!("WebSocket connection established");
        
    let mut ws_session = WebSocketSession::new(session);

    let consumer: StreamConsumer = ClientConfig::new()
    .set("bootstrap.servers", "localhost:9092")
    .set("group.id", "test_group")
    .set("auto.offset.reset", "earliest")
    .create().unwrap();

    // Handle this WebSocket and kafka event in the current task
    actix_web::rt::spawn(async move {
        consumer.subscribe(&["demo"]).unwrap();
        let mut kafka_stream = consumer.stream();
        loop {
            tokio::select! {
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(msg2)) => {
                            ws_session.handle_message(msg2).await;
                        }
                        Some(Err(e)) => {
                            println!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            println!("WebSocket clsoed");
                            break;
                        }
                    }
                }

                kafkamsg = kafka_stream.next() => {              
                    match kafkamsg {
                        Some(Ok(msg)) => {                 
                            if let (Some(key), Some(payload)) = (msg.key(), msg.payload()) {
                                match serde_json::from_slice::<KLineData>(payload) {
                                    Ok(data) => {
                                        let sub_key = String::from_utf8_lossy(key);
                                        // println!("kline data {:?}:" , sub_key );
                                        // println!("  open: {}", data.open);
                                        // println!("  high: {}", data.high);
                                        // println!("  low: {}", data.low);
                                        // println!("  close: {}", data.close);
                                        // println!("  unix_timestamp: {}", data.unix_timestamp);


                                        if ws_session.subscriptions.as_ref().map_or(false, |s| s == &sub_key) {                                            
                                            ws_session.text(&String::from_utf8_lossy(payload)).await
                                        }                                        
                                    },
                                    Err(e) => {
                                        eprintln!("deserialized : {}", e);
                                    }
                                }
                                
                            }
                        }
                        Some(Err(e)) => {
                            println!("kafka error: {}", e);                            
                        }
                        None => {
                            println!("kafka is None");                            
                            break;
                        }
                    }
                }                
            }
        }
        println!("Kafka consumer task ended");
    });
    
    Ok(response)
}

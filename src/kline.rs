use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use rdkafka::producer::FutureProducer;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KLineData {    
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,    

    pub unix_timestamp : u64,
    pub time_window_seconds : u64,
}

impl KLineData {
    pub fn new(start_unixtime : u64, time_window : u64) -> Self {
        
        Self {            
            open: 0f64,
            high: 0f64,
            low: 0f64,
            close: 0f64,
            unix_timestamp : start_unixtime,
            time_window_seconds : time_window,
        }
    }

    pub fn update(&mut self, price: f64, timestamp : u64) {
        if timestamp > (self.unix_timestamp + self.time_window_seconds) {
            self.open = price;
            self.high = price;
            self.low = price;
            self.close = price;
            self.unix_timestamp += self.time_window_seconds;            
        } else {
            self.high = self.high.max(price);
            self.low = self.low.min(price);
            self.close = price;
        }
        
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeWindow {
    OneSecond = 1,
    OneMinute = 60,
    FiveMinutes = 300,
    OneHour = 3600,
}

impl TimeWindow {
    pub fn all() -> Vec<TimeWindow> {
        vec![
            TimeWindow::OneSecond,
            TimeWindow::OneMinute,
            TimeWindow::FiveMinutes,
            TimeWindow::OneHour,
        ]
    }
}

pub struct KLineAggregator {
    data: HashMap<TimeWindow, KLineData>,
}

impl KLineAggregator {
    pub fn new() -> Self {
        let mut data = HashMap::new();
        let unix_time_secs = Utc::now().timestamp() as u64;
        
        for window in TimeWindow::all() {
            let t = ( unix_time_secs / (window as u64) - 1 ) * window as u64;
            data.insert(window, KLineData::new(t, window as u64));
        }
        Self { data }
    }

    pub fn get_klines(&self, window: TimeWindow) -> &KLineData {
        self.data.get(&window).unwrap()
    }

    pub fn get_all_klines(&self) -> Vec<KLineData> {
        self.data.values().cloned().collect()
    }

    pub fn update(&mut self, price : f64, current_timestamp : u64) {
        for window in TimeWindow::all() {            
            self.data.get_mut(&window).unwrap().update(price, current_timestamp);
        }
    }

    pub fn send_to_kafka_for_each<F>(&self, mut func: F, producer: &FutureProducer)
    where
        F: FnMut(&FutureProducer, &KLineData) -> Result<(), Box<dyn Error>>
    {
        self.data.iter().for_each(|(_,k)| {let _ = func(producer, k); });
    }
   

    pub fn print_current_status(&self) {
        println!("\n=== Current K-Line Status ===");
        for window in TimeWindow::all() {
            let window_data = self.data.get(&window);            
            
            if let Some(current_kline) = window_data {
                println!("{:?}: start_time:{:?} O:{:.12} H:{:.12} L:{:.12} C:{:.12}", 
                    window,
                    current_kline.unix_timestamp,
                    current_kline.open,
                    current_kline.high,
                    current_kline.low,
                    current_kline.close,
                );
            }
        }
        println!("=============================\n");
    }
}

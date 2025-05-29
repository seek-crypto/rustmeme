# K-Line WebSocket Service

This Rust application receives mock PumpFun messages, aggregates them into K-line data, and the send to kafka broker, and provides a WebSocket interface for clients to access real-time K-line data.

## Features

- **Real-time Data Processing**: Receives mock PumpFun messages and processes them into K-line data
- **Multiple Time Windows**: Supports 1-second, 1-minute, 5-minute, and 1-hour K-line aggregation
- **WebSocket API**: Provides real-time K-line data via WebSocket connections
- **Web Client**: Includes a HTML client (websocket_client.html) for testing and visualization
- **Kakfka** : Support thousands of client(kafka consumer) through kafka 

## Improvements

- Instead of using JSON, employ a binary data format similar to Protobuf. JSON serialization/deserialization is extremely resource-intensive. As is well known, Solana does not use JSON and instead uses binary-formatted data.
- We can use io_uring to optimize network I/O performance, but I don't think this is a top priority.
- Reduce the use of unwrap() and improve error handling.
- Since web page refreshes are intended for human viewing and don't need to happen many times per second, it's sufficient to push the latest K-line data to the web client once every second.  So that a separate Kafka consumer is no longer assigned to each WebSocket client.


## Architecture

### Components

1. **Main Application** (`src/main.rs`):
   - Runs message processing and WebSocket server concurrently
   - Processes mock PumpFun messages from a channel
   - Send real-time K-Line data to a Kafka broker. The topic is "demo", and the message keys follow the format "key_1", "key_3600", etc. The payload contains the current K-line price, timestamp, and other data.

2. **K-Line Aggregator** (`src/kline.rs`):
   - Aggregates price data into K-line format (OHLCV)
   - Supports multiple time windows simultaneously

3. **WebSocket Service** (`src/websocket.rs`):
   - Handles WebSocket connections
   - For each WebSocket client connection, a dedicated Kafka consumer is created (ref code: pub async fn websocket_handler ). If the client sends a subscription command like sub_5m, the server will forward Kafka messages with the corresponding key (e.g., "key_300" and json payload of kline) to the client. The client can unsubscribe using the unsub command.
   - Supports subscription/unsubscription to different time windows
   - Provides real-time updates and historical data retrieval

4. **Mock Data Source** (`src/mock_source.rs`):
   - Generates random mock PumpFun messages (virtual_token_reserves, virtual_sol_reserves) for testing and sends them to task in main.rs via a channel.

## Usage

### Running the Application

```bash

# start local kafka by docker 
docker run -d --name=kafka -p 9092:9092 apache/kafka

# Build the project
cargo build

# Run the main application
cargo run

# open browser 
chrome http://127.0.0.1:8080/

click button "sub-5m" to subscribe k-line data 
click button "unsub" to unsubscribe 

```

The application will:
- Start generating mock PumpFun messages
- Begin processing and aggregating K-line data
- Start the WebSocket server on `http://127.0.0.1:8080`

### WebSocket API

Connect to: `ws://127.0.0.1:8080/ws`

#### Message Types

**Subscribe to a time window And un-subscribe:**
```json
  "sub-1s" | "sub-1m" | "sub-5m" | "sub-1h",
  "unsub"
```

#### Response Messages

**K-line Update:**
```json
{ 
   "open": 0.001, 
   "high": 0.0010010979158340965, 
   "low": 0.001, 
   "close": 0.001001096106439179, 
   "unix_timestamp": 1748465400, 
   "time_window_seconds": 300 
}
```

## Data Flow

1. **Mock Data Generation**: `mock_source.rs` generates random PumpFun messages
2. **Message Processing**: Main thread receives messages and calculates price
3. **K-line Aggregation**: Data is aggregated into K-lines for all time windows
4. **Kakfa Producder**:  Send K-lines data to kakfa broker
6. **WebSocket Task/Kafka Consumer**: Real-time K-Line updates from Kafka topic "demo" are sent to subscribed clients

## File Structure

```
├── src/
│   ├── main.rs           # Main application entry point 
│   ├── kline.rs          # K-line data structures and aggregation
│   ├── websocket.rs      # WebSocket server implementation
│   ├── mock_source.rs    # Mock data generation
│   └── lib.rs            # Library exports
├── websocket_client.html # Web client for testing
├── Cargo.toml           # Project dependencies
└── README.md            # This file
```

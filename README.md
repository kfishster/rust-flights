# 🦀 Rust Flights

A high-performance Rust library that reverse engineers the Google Flights API. This library provides better performance than existing Python implementations while maintaining API compatibility.

[![Crates.io](https://img.shields.io/crates/v/rust-flights.svg)](https://crates.io/crates/rust-flights)
[![Documentation](https://docs.rs/rust-flights/badge.svg)](https://docs.rs/rust-flights)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ✨ Features

- **High Performance**: Native Rust implementation with async/await support
- **Python API Compatibility**: Drop-in replacement for existing Python flight scrapers
- **Rich Flight Data**: Comprehensive flight information including prices, durations, stops, and delays
- **Flexible Search**: Support for one-way, round-trip, and multi-city searches
- **CLI Interface**: Command-line tool for quick flight searches
- **Type Safety**: Full Rust type safety with comprehensive error handling

## 🚀 Quick Start

### Library Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-flights = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Example

```rust
use rust_flights::{get_flights, FlightData, FlightSearchRequest, Passengers, SeatClass, TripType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create flight search request
    let request = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2024-03-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: Some(vec!["AA".to_string(), "DL".to_string()]),
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(), // 1 adult
        seat_class: SeatClass::Economy,
    };
    
    // Search for flights
    let result = get_flights(request).await?;
    
    println!("Found {} flights", result.flights.len());
    for flight in result.flights.iter().take(3) {
        println!("{}: {} - {}", flight.name, flight.departure, flight.price);
    }
    
    Ok(())
}
```

### CLI Usage

Install the CLI tool:

```bash
cargo install rust-flights --features cli
```

Search for flights:

```bash
# One-way flight
rust-flights search --from LAX --to JFK --date 2024-03-15

# Round-trip flight
rust-flights search --from LAX --to JFK --date 2024-03-15 --return-date 2024-03-20

# With specific preferences
rust-flights search \
  --from LAX \
  --to JFK \
  --date 2024-03-15 \
  --adults 2 \
  --children 1 \
  --class business \
  --max-stops 0 \
  --airlines "AA,DL" \
  --output flights.json
```

## 📖 API Reference

### Core Types

#### `FlightData`
```rust
pub struct FlightData {
    pub date: String,              // YYYY-MM-DD format
    pub from_airport: String,      // Airport code (e.g., "LAX")
    pub to_airport: String,        // Airport code (e.g., "JFK")
    pub max_stops: Option<i32>,    // Maximum number of stops
    pub airlines: Option<Vec<String>>, // Preferred airlines
}
```

#### `Passengers`
```rust
pub struct Passengers {
    pub adults: i32,
    pub children: i32,
    pub infants_in_seat: i32,
    pub infants_on_lap: i32,
}
```

#### `FlightResult`
```rust
pub struct FlightResult {
    pub current_price: String,    // "low", "typical", "high"
    pub flights: Vec<Flight>,
}
```

#### `Flight`
```rust
pub struct Flight {
    pub is_best: bool,
    pub name: String,             // Airline name
    pub departure: String,        // Departure time
    pub arrival: String,          // Arrival time
    pub arrival_time_ahead: String,
    pub duration: String,         // Flight duration
    pub stops: i32,              // Number of stops
    pub delay: Option<String>,   // Delay information
    pub price: String,           // Price
}
```

### Main Function

```rust
pub async fn get_flights(request: FlightSearchRequest) -> Result<FlightResult, FlightError>
```

#### FlightSearchRequest

```rust
pub struct FlightSearchRequest {
    pub flights: Vec<FlightData>,
    pub trip_type: TripType,     // OneWay, RoundTrip, MultiCity
    pub passengers: Passengers,
    pub seat_class: SeatClass,   // Economy, PremiumEconomy, Business, First
}
```

## 🏗️ Architecture

The library is structured into four main modules:

1. **`lib.rs`** - Core types and public API
2. **`protobuf.rs`** - Protocol buffer definitions and encoding
3. **`client.rs`** - HTTP client and HTML parsing
4. **`main.rs`** - CLI interface (optional)

### How It Works

1. **Protobuf Encoding**: Flight search parameters are encoded into Google's protobuf format
2. **Base64 Encoding**: The protobuf is base64-encoded for the URL parameter
3. **HTTP Request**: Sends GET request to `https://www.google.com/travel/flights?tfs=<encoded>`
4. **HTML Parsing**: Parses the returned HTML using CSS selectors
5. **Data Extraction**: Extracts flight information into structured data

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with CLI features
cargo test --features cli

# Run clippy for linting
cargo clippy

# Run example
cargo run --example basic_search
```

## ⚠️ Important Notes

- This library reverse engineers Google Flights and may break if Google changes their API
- Use responsibly and respect Google's Terms of Service
- Consider implementing rate limiting to avoid being blocked
- The library is for educational and research purposes

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the Python `fast-flights` library
- Built with the amazing Rust ecosystem

## ⚡️ MCP Server

This project includes a **Model Context Protocol (MCP) server** that exposes the flight search functionality as a tool for Large Language Models (LLMs) like Claude. The MCP server allows AI assistants to search for flights on behalf of users using natural language requests.

### ✅ Features

- **Unified Flight Search**: Single tool that handles both airport codes (e.g., LAX, JFK) and city names (e.g., Los Angeles, New York)
- **Comprehensive Parameters**: Support for all flight search options including passengers, seat class, time windows, airlines, and trip types
- **Smart Mode Selection**: Automatically routes to airport-based or city-based search based on input parameters
- **Error Handling**: Graceful error messages for invalid inputs or API failures
- **Async Support**: Built with Rust's async/await for high performance

### How it Works

The MCP server runs as a separate binary (`rust-flights-mcp`) and communicates with MCP clients over `stdio` transport. It exposes a single `get_flights` tool that accepts either:

- **Airport Search**: Specify `airports` with `from_airport` and `to_airport` codes
- **City Search**: Specify `cities` with `from_city` and `to_city` names (uses Wikidata integration)

### Building and Running the MCP Server

```bash
# Build the MCP server binary
cargo build --bin rust-flights-mcp

# Run the server
./target/debug/rust-flights-mcp
```

### Testing with MCP Inspector

You can test the server using the official MCP Inspector:

```bash
# Install the MCP Inspector
npx @modelcontextprotocol/inspector ./target/debug/rust-flights-mcp
```

Then navigate to `http://127.0.0.1:6274` in your browser to interact with the server.

### Integration with Claude Desktop

Add the following to your Claude Desktop configuration file (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "rust-flights": {
      "command": "/absolute/path/to/rust-flights/target/debug/rust-flights-mcp",
      "args": []
    }
  }
}
```

### Example Tool Usage

The `get_flights` tool accepts the following parameters:

**Airport-based search:**
```json
{
  "airports": {
    "from_airport": "LAX",
    "to_airport": "JFK"
  },
  "departure_date": "2024-03-15",
  "adults": 2,
  "seat_class": "economy",
  "trip_type": "round-trip",
  "return_date": "2024-03-20"
}
```

**City-based search:**
```json
{
  "cities": {
    "from_city": "Los Angeles",
    "to_city": "New York"
  },
  "departure_date": "2024-03-15",
  "adults": 1,
  "seat_class": "business"
}
```

**Additional Parameters:**
- `children`, `infants_in_seat`, `infants_on_lap`: Passenger counts
- `max_stops`: Maximum number of stops (0, 1, 2, 3)
- `airlines`: Array of preferred airline codes (e.g., ["AA", "DL"])
- `departure_time`, `arrival_time`: Time windows in HH:MM-HH:MM format
- `trip_type`: "one-way" or "round-trip"

### Tool Output

The tool returns formatted flight results including:
- Number of flights found and current price level
- Best or top flights with details:
  - Airline name and flight details
  - Departure and arrival times
  - Flight duration and number of stops
  - Price information
  - Delay information (if any)

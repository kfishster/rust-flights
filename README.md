# 🦀✈️ Rust Flights - Blazingly Fast Google Flights API

> **Lightning-fast flight search powered by Rust!** 🚀 

A high-performance Rust library that reverse engineers the Google Flights API, now with **city name support** and **MCP server integration** for AI assistants! This library delivers blazing performance while maintaining full API compatibility with existing Python implementations.

[![Crates.io](https://img.shields.io/crates/v/rust-flights.svg)](https://crates.io/crates/rust-flights)
[![Documentation](https://docs.rs/rust-flights/badge.svg)](https://docs.rs/rust-flights)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ Important Disclaimer

**This library works by reverse engineering and scraping Google Flights.** If Google changes their website structure or API, this library may break until updated. Use responsibly and respect Google's Terms of Service! 🙏

## ✨ What Makes This Special?

🔥 **Blazing Performance**: Native Rust implementation with async/await  
🌍 **Smart City Search**: Uses Wikidata to resolve city names to airports automatically  
🤖 **AI Assistant Ready**: Built-in MCP server for Claude Desktop, Cursor, and more!  
📅 **Comprehensive Search**: One-way, round-trip, and multi-city flights  
⏰ **Time Filtering**: Departure and arrival time windows  
🎯 **Type Safety**: Full Rust type safety with comprehensive error handling  
💾 **Intelligent Caching**: Local city cache for lightning-fast lookups  
🛠️ **CLI & Library**: Use as a library or command-line tool  

## 🚀 Quick Start

### 📦 Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rust-flights = { path = <path to local version of this repo>}
tokio = { version = "1.0", features = ["full"] }
```

### 🌟 Basic Example - Airport Search

```rust
use rust_flights::{
    get_flights, FlightData, FlightSearchRequest, 
    Passengers, SeatClass, TripType
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Search flights from LAX to JFK
    let request = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2024-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: Some(vec!["AA".to_string(), "DL".to_string()]),
            departure_time: None,
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(), // 1 adult
        seat_class: SeatClass::Economy,
    };
    
    let result = get_flights(request).await?;
    
    println!("🎉 Found {} flights!", result.flights.len());
    for flight in result.flights.iter().take(3) {
        println!("✈️  {}: {} → {} ({})", 
            flight.name, flight.departure, flight.arrival, 
            flight.price.amount
        );
    }
    Ok(())
}
```

### 🏙️ City Search Example

```rust
use rust_flights::{
    get_flights_by_city, CityFlightData, CityFlightSearchRequest,
    Passengers, SeatClass, TripType
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Search flights from London to Tokyo using city names!
    let request = CityFlightSearchRequest {
        flights: vec![CityFlightData {
            date: "2024-03-15".to_string(),
            from_city: "London".to_string(),
            to_city: "Tokyo".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: None,
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Business,
    };
    
    let result = get_flights_by_city(request).await?;
    
    println!("🌍 Found {} flights from London to Tokyo!", result.flights.len());
    for flight in result.flights.iter().take(3) {
        println!("🗾 {}: {} → {} ({})", 
            flight.name, flight.departure, flight.arrival,
            flight.price.amount
        );
    }
    Ok(())
}
```

## 🤖 AI Assistant Integration (MCP Server)

The **coolest feature** - integrate with your favorite AI assistants! 🎉

### 🔧 Building the MCP Server

```bash
# Build both the library and MCP server
cargo build --release

# The MCP server binary will be at:
# ./target/release/rust-flights-mcp
```

### 🧠 Claude Desktop Setup

Add this to your `claude_desktop_config.json`:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "rust-flights": {
      "command": "/absolute/path/to/your/rust-flights/target/release/rust-flights-mcp",
      "args": []
    }
  }
}
```

### 🆚 Cursor Setup

Add to your Cursor settings:

1. Open Cursor Settings (Cmd/Ctrl + ,)
2. Search for "MCP"
3. Add server configuration:

```json
{
  "mcp.servers": {
    "rust-flights": {
      "command": "/absolute/path/to/your/rust-flights/target/release/rust-flights-mcp"
    }
  }
}
```

### 🐙 GitHub Copilot Chat Setup

Configure in your VS Code settings.json:

```json
{
  "github.copilot.chat.mcp.servers": {
    "rust-flights": {
      "command": "/absolute/path/to/your/rust-flights/target/release/rust-flights-mcp"
    }
  }
}
```

### 🧪 Testing Your MCP Server

Use the official MCP Inspector:

```bash
npx @modelcontextprotocol/inspector ./target/release/rust-flights-mcp
```

Then visit `http://127.0.0.1:6274` to interact with your server! 🎮

## 🎭 MCP Tools Available

Your AI assistant will have access to these tools:

### 🔍 `get_flights` - Unified Flight Search
- **Airport Search**: Use airport codes (LAX, JFK, LHR)
- **City Search**: Use city names (Los Angeles, New York, London)
- **Smart Routing**: Automatically detects search type
- **All Features**: Time windows, passenger counts, seat classes

### 🔗 `get_itinerary_link` - Generate Booking Links
- Creates Google Flights URLs for specific flights
- Perfect for booking the flights you found!

## 💬 Example AI Conversations

Once set up, you can ask your AI assistant:

> "Find me flights from San Francisco to Tokyo on August 15th, business class"

> "Show me flights from London to New York next week, I want to arrive in the morning"

> "Find the cheapest flights from Los Angeles to Miami with at most 1 stop"

The AI will use the MCP tools to search and present results beautifully! 🎨

## 🔧 CLI Usage

Install the CLI tool:

```bash
cargo install rust-flights --features cli
```

### ✈️ Airport Search
```bash
# One-way flight
rust-flights search --from LAX --to JFK --date 2024-03-15

# Round-trip with preferences
rust-flights search \
  --from LAX --to JFK \
  --date 2024-03-15 --return-date 2024-03-22 \
  --adults 2 --children 1 \
  --class business --max-stops 0 \
  --airlines "AA,DL" --output flights.json
```

### 🏙️ City Search
```bash
# Search by city names
rust-flights city-search \
  --from-city "Los Angeles" --to-city "New York" \
  --date 2024-03-15 --class economy

# Quick city search
rust-flights quick-city "London" "Paris" 2024-03-15
```

## 📚 API Reference

### 🏗️ Core Types

#### `FlightSearchRequest` - Airport-based search
```rust
pub struct FlightSearchRequest {
    pub flights: Vec<FlightData>,      // Flight segments
    pub trip_type: TripType,           // OneWay, RoundTrip, MultiCity
    pub passengers: Passengers,        // Passenger counts
    pub seat_class: SeatClass,         // Economy, Business, etc.
}
```

#### `CityFlightSearchRequest` - City-based search
```rust
pub struct CityFlightSearchRequest {
    pub flights: Vec<CityFlightData>,  // City flight segments
    pub trip_type: TripType,
    pub passengers: Passengers,
    pub seat_class: SeatClass,
}
```

#### `FlightData` vs `CityFlightData`
```rust
pub struct FlightData {
    pub date: String,                  // YYYY-MM-DD
    pub from_airport: String,          // "LAX"
    pub to_airport: String,            // "JFK"
    pub max_stops: Option<i32>,
    pub airlines: Option<Vec<String>>,
    pub departure_time: Option<TimeWindow>,
    pub arrival_time: Option<TimeWindow>,
}

pub struct CityFlightData {
    pub date: String,                  // YYYY-MM-DD
    pub from_city: String,             // "Los Angeles"
    pub to_city: String,               // "New York"
    // ... same other fields
}
```

#### `TimeWindow` - Time filtering
```rust
pub struct TimeWindow {
    pub earliest_hour: i32,            // 0-23
    pub latest_hour: i32,              // 0-23
}

// Create from string: "06:00-12:00"
let window = TimeWindow::from_range_str("06:00-12:00")?;
```

#### `FlightResult` - Search results
```rust
pub struct FlightResult {
    pub current_price: String,         // "low", "typical", "high"
    pub flights: Vec<Flight>,
}

pub struct Flight {
    pub is_best: bool,
    pub name: String,                  // Airline name
    pub departure: String,             // Departure time
    pub arrival: String,               // Arrival time
    pub duration: String,              // Flight duration
    pub stops: i32,                    // Number of stops
    pub price: FlightPrice,            // Price with currency
    pub airline_code: Option<String>,  // "AA", "DL"
    pub flight_number: Option<String>, // "1234"
    // ... more fields
}
```

### 🎯 Main Functions

```rust
// Airport search
pub async fn get_flights(request: FlightSearchRequest) -> Result<FlightResult, FlightError>

// City search (with Wikidata integration)
pub async fn get_flights_by_city(request: CityFlightSearchRequest) -> Result<FlightResult, FlightError>

// Quick city search
pub async fn search_flights_between_cities(
    from_city: &str, 
    to_city: &str, 
    date: &str
) -> Result<FlightResult, FlightError>
```

## 🏛️ Architecture Deep Dive

### 🧩 Module Structure
```
src/
├── lib.rs          # 📝 Public API and core types
├── client.rs       # 🌐 HTTP client and HTML parsing  
├── protobuf.rs     # 📦 Google's protobuf encoding
├── wikidata.rs     # 🌍 City-to-airport resolution
├── mcp_server.rs   # 🤖 MCP server implementation
├── main.rs         # 💻 CLI interface
```

### 🔄 How It Works

1. **🔍 Input Processing**: Detect airport codes vs city names
2. **🌍 City Resolution**: Query Wikidata to find airport codes (cached locally!)
3. **📦 Protobuf Magic**: Encode search parameters into Google's format
4. **🔗 Base64 Encoding**: Convert to URL-safe format
5. **🌐 HTTP Request**: Send GET request to Google Flights
6. **🎯 HTML Parsing**: Extract flight data using CSS selectors
7. **✨ Data Transformation**: Convert to structured results

### 🧠 City Intelligence

The Wikidata integration is **smart**:
- 💾 **Local Cache**: Pre-populated with 200+ popular cities
- 🔍 **Fuzzy Matching**: "New York" matches "New York City"
- 🌐 **Fallback Queries**: Live Wikidata lookup for cache misses
- ⚡ **Lightning Fast**: Cached lookups are instant

### 🤖 MCP Integration

The MCP server exposes:
- 🔧 **Unified Tool**: Single `get_flights` tool for both search modes
- 🔗 **Itinerary Generation**: Create booking links from search results
- 📊 **Rich Responses**: Formatted flight data for AI consumption
- ⚡ **Async Performance**: Non-blocking operations

## 🧪 Testing & Development

```bash
# Run all tests
cargo test

# Run with CLI features
cargo test --features cli

# Run clippy for linting
cargo clippy

# Test MCP server with inspector
npx @modelcontextprotocol/inspector ./target/debug/rust-flights-mcp

# Build optimized release
cargo build --release
```

## 🤝 Contributing

We'd love your help making this even more awesome! 🌟

- 🐛 **Bug Reports**: Found something broken? Let us know!
- 💡 **Feature Ideas**: Have a cool idea? Open an issue!
- 🔧 **Pull Requests**: Code contributions are super welcome!
- 📚 **Documentation**: Help make the docs even better!

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- 🐍 Inspired by the Python `fast-flights` library
- 🦀 Built with the amazing Rust ecosystem
- 🌍 Powered by Wikidata for city resolution
- 🤖 MCP integration via the official Rust SDK

Happy flying! ✈️🦀✨

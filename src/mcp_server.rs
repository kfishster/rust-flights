// src/mcp_server.rs

use rmcp::{
    ServerHandler, ServiceExt,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
    transport::stdio,
};
use rust_flights::{
    get_flights as get_flights_internal, get_flights_by_city as get_flights_by_city_internal,
    CityFlightData, CityFlightSearchRequest, FlightData, FlightResult, FlightSearchRequest,
    Passengers, SeatClass, TimeWindow, TripType, SelectedFlight, build_itinerary_info, encode_to_base64,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender;
use std::path::PathBuf;

/// Flight search MCP server
#[derive(Default, Clone)]
pub struct FlightServer;

impl FlightServer {
    pub fn new() -> Self {
        Self
    }

    /// Initialize logging to file
    fn init_logging() -> Result<()> {
        // Create logs directory if it doesn't exist
        let log_dir = PathBuf::from("logs");
        std::fs::create_dir_all(&log_dir)?;
        
        // Create a file appender for rotating logs - using blocking writer for simplicity
        let file_appender = tracing_appender::rolling::daily(&log_dir, "rust-flights-mcp.log");
        
        // Set up the subscriber with file output - forced debug level
        tracing_subscriber::registry()
            .with(
                EnvFilter::new("debug") // Force debug level for all modules
                    .add_directive("rust_flights=debug".parse()?)
                    .add_directive("reqwest=trace".parse()?)  // Keep external libs at trace to see everything
                    .add_directive("hyper=trace".parse()?)
                    .add_directive("h2=trace".parse()?)
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(file_appender)  // Use blocking writer directly
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .json() // Structured JSON logging for easier parsing
            )
            .init();
        
        info!("Logging initialized - logs will be written to logs/rust-flights-mcp.log.*");
        debug!("Debug logging is enabled and working");
        Ok(())
    }
}

/// Unified flight search parameters with explicit mode selection
#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
pub struct FlightSearchParams {
    // Airport search parameters
    #[schemars(description = "Origin airport code (e.g., LAX, JFK) - use for airport-based search")]
    pub from_airport: Option<String>,
    #[schemars(description = "Destination airport code (e.g., JFK, LHR) - use for airport-based search")]
    pub to_airport: Option<String>,
    // City search parameters
    #[schemars(description = "Origin city name (e.g., Los Angeles, New York) - use for city-based search")]
    pub from_city: Option<String>,
    #[schemars(description = "Destination city name (e.g., New York, London) - use for city-based search")]
    pub to_city: Option<String>,
    // Common search parameters
    #[schemars(description = "Departure date in YYYY-MM-DD format")]
    pub departure_date: String,
    #[schemars(description = "Return date in YYYY-MM-DD format for round trips")]
    pub return_date: Option<String>,
    #[schemars(description = "Number of adult passengers")]
    pub adults: Option<i32>,
    #[schemars(description = "Number of child passengers")]
    pub children: Option<i32>,
    #[schemars(description = "Number of infants in seat")]
    pub infants_in_seat: Option<i32>,
    #[schemars(description = "Number of infants on lap")]
    pub infants_on_lap: Option<i32>,
    #[schemars(description = "Seat class: economy, premium-economy, business, first")]
    pub seat_class: Option<String>,
    #[schemars(description = "Maximum number of stops")]
    pub max_stops: Option<i32>,
    #[schemars(description = "Preferred airlines (comma-separated, e.g., 'AA,DL,UA')")]
    pub airlines: Option<String>,
    #[schemars(description = "Departure time window in HH:MM-HH:MM format")]
    pub departure_time: Option<String>,
    #[schemars(description = "Arrival time window in HH:MM-HH:MM format")]
    pub arrival_time: Option<String>,
    #[schemars(description = "Trip type: one-way or round-trip")]
    pub trip_type: Option<String>,
    #[schemars(description = "Maximum number of flights to return (default: 30)")]
    pub max_flights: Option<usize>,
}

/// Selected flight information for itinerary links
#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
pub struct SelectedFlightInfo {
    #[schemars(description = "Origin airport code (e.g., LAX, JFK)")]
    pub from_airport: String,
    #[schemars(description = "Destination airport code (e.g., JFK, LHR)")]
    pub to_airport: String,
    #[schemars(description = "Departure date in YYYY-MM-DD format")]
    pub departure_date: String,
    #[schemars(description = "Airline code (e.g., AA, DL, UA)")]
    pub airline_code: String,
    #[schemars(description = "Flight number (e.g., 1234)")]
    pub flight_number: String,
}

/// Itinerary link request parameters
#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
pub struct ItineraryRequest {
    #[schemars(description = "List of selected flights for the itinerary")]
    pub flights: Vec<SelectedFlightInfo>,
    #[schemars(description = "Number of adult passengers")]
    pub adults: Option<i32>,
    #[schemars(description = "Number of child passengers")]
    pub children: Option<i32>,
    #[schemars(description = "Number of infants in seat")]
    pub infants_in_seat: Option<i32>,
    #[schemars(description = "Number of infants on lap")]
    pub infants_on_lap: Option<i32>,
    #[schemars(description = "Seat class: economy, premium-economy, business, first")]
    pub seat_class: Option<String>,
    #[schemars(description = "Trip type: one-way or round-trip")]
    pub trip_type: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FlightInfo {
    pub airline_name: String,
    pub departure_time: String,
    pub arrival_time: String,
    pub duration: String,
    pub origin: String,
    pub destination: String,
    pub stops: i32,
    pub price: rust_flights::FlightPrice,
    pub airline_code: Option<String>,
    pub flight_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FlightSearchResult {
    pub total_flights: usize,
    pub current_price_level: String,
    pub best_flights: Vec<FlightInfo>,
    pub other_flights: Vec<FlightInfo>,
}

#[tool(tool_box)]
impl FlightServer {
    /// Unified flight search with explicit mode selection
    #[tool(description = "Search for flights between locations. Specify either airport codes (from_airport/to_airport) or city names (from_city/to_city), but not both.")]
    async fn get_flights(
        &self,
        #[tool(aggr)] params: FlightSearchParams,
    ) -> String {
        let max_flights = params.max_flights;
        
        // Log the incoming request
        info!(
            from_airport = params.from_airport.as_deref(),
            to_airport = params.to_airport.as_deref(),
            from_city = params.from_city.as_deref(),
            to_city = params.to_city.as_deref(),
            departure_date = params.departure_date,
            return_date = params.return_date.as_deref(),
            adults = params.adults.unwrap_or(1),
            children = params.children.unwrap_or(0),
            seat_class = params.seat_class.as_deref().unwrap_or("economy"),
            trip_type = params.trip_type.as_deref().unwrap_or("one-way"),
            max_flights = max_flights.unwrap_or(30),
            "Flight search request received"
        );
        
        let result = match (&params.from_airport, &params.to_airport, &params.from_city, &params.to_city) {
            // Airport-based search
            (Some(from_airport), Some(to_airport), None, None) => {
                info!(from_airport = from_airport, to_airport = to_airport, "Using airport-based search");
                match build_flight_search_request(from_airport.clone(), to_airport.clone(), params.clone()) {
                    Ok(request) => {
                        debug!("Built flight search request successfully");
                        get_flights_internal(request).await
                    },
                    Err(e) => {
                        error!("Error building flight request: {}", e);
                        return format!(r#"{{"error": "Error building flight request: {}"}}"#, e);
                    }
                }
            }
            // City-based search
            (None, None, Some(from_city), Some(to_city)) => {
                info!(from_city = from_city, to_city = to_city, "Using city-based search");
                match build_city_flight_search_request(from_city.clone(), to_city.clone(), params.clone()) {
                    Ok(request) => {
                        debug!("Built city flight search request successfully");
                        get_flights_by_city_internal(request).await
                    },
                    Err(e) => {
                        error!("Error building city flight request: {}", e);
                        return format!(r#"{{"error": "Error building city flight request: {}"}}"#, e);
                    }
                }
            }
            // Invalid combinations
            (Some(_), None, _, _) | (None, Some(_), _, _) => {
                warn!("Invalid airport search parameters - missing from_airport or to_airport");
                return r#"{"error": "For airport search, both from_airport and to_airport must be specified"}"#.to_string()
            }
            (_, _, Some(_), None) | (_, _, None, Some(_)) => {
                warn!("Invalid city search parameters - missing from_city or to_city");
                return r#"{"error": "For city search, both from_city and to_city must be specified"}"#.to_string()
            }
            (Some(_), Some(_), Some(_), Some(_)) => {
                warn!("Invalid search parameters - both airport codes and city names specified");
                return r#"{"error": "Cannot specify both airport codes and city names - choose one search mode"}"#.to_string()
            }
            (None, None, None, None) => {
                warn!("Invalid search parameters - no location information provided");
                return r#"{"error": "Must specify either airport codes (from_airport/to_airport) or city names (from_city/to_city) for flight search"}"#.to_string()
            }
        };

        match result {
            Ok(flight_result) => {
                info!(
                    flights_found = flight_result.flights.len(),
                    current_price_level = flight_result.current_price,
                    "Flight search completed successfully"
                );
                format_flight_results_json(flight_result, max_flights)
            },
            Err(e) => {
                error!("Flight search failed: {}", e);
                format!(r#"{{"error": "Flight search failed: {}"}}"#, e)
            }
        }
    }

    /// Generate a Google Flights itinerary link for selected flights
    #[tool(description = "Generate a Google Flights itinerary link for specific selected flights. Provide flight details including departure date, airline code, and flight number for each flight.")]
    async fn get_itinerary_link(
        &self,
        #[tool(aggr)] params: ItineraryRequest,
    ) -> String {
        info!(
            flights_count = params.flights.len(),
            adults = params.adults.unwrap_or(1),
            children = params.children.unwrap_or(0),
            seat_class = params.seat_class.as_deref().unwrap_or("economy"),
            trip_type = params.trip_type.as_deref().unwrap_or("one-way"),
            "Itinerary link request received"
        );

        if params.flights.is_empty() {
            warn!("Empty flights list provided for itinerary link generation");
            return r#"{"error": "At least one flight must be specified"}"#.to_string();
        }

        // Convert to internal SelectedFlight format
        let selected_flights: Vec<SelectedFlight> = params.flights.into_iter().map(|f| {
            debug!(
                from_airport = f.from_airport, 
                to_airport = f.to_airport,
                airline_code = f.airline_code,
                flight_number = f.flight_number,
                "Processing flight for itinerary"
            );
            SelectedFlight {
                from_airport: f.from_airport,
                to_airport: f.to_airport,
                departure_date: f.departure_date,
                airline_code: f.airline_code,
                flight_number: f.flight_number,
            }
        }).collect();

        // Build passenger configuration
        let passengers = Passengers {
            adults: params.adults.unwrap_or(1),
            children: params.children.unwrap_or(0),
            infants_in_seat: params.infants_in_seat.unwrap_or(0),
            infants_on_lap: params.infants_on_lap.unwrap_or(0),
        };

        // Parse trip type
        let trip_type = match params.trip_type.as_deref().unwrap_or("one-way").parse::<TripType>() {
            Ok(tt) => {
                debug!("Successfully parsed trip type: {:?}", tt);
                tt
            },
            Err(e) => {
                error!("Invalid trip type: {}", e);
                return format!(r#"{{"error": "Invalid trip type: {}"}}"#, e);
            }
        };

        // Parse seat class
        let seat_class = match params.seat_class.as_deref().unwrap_or("economy").parse::<SeatClass>() {
            Ok(sc) => {
                debug!("Successfully parsed seat class: {:?}", sc);
                sc
            },
            Err(e) => {
                error!("Invalid seat class: {}", e);
                return format!(r#"{{"error": "Invalid seat class: {}"}}"#, e);
            }
        };

        // Build protobuf info
        let info = match build_itinerary_info(selected_flights, trip_type, passengers, seat_class) {
            Ok(info) => {
                debug!("Successfully built protobuf itinerary info");
                info
            },
            Err(e) => {
                error!("Error building itinerary: {}", e);
                return format!(r#"{{"error": "Error building itinerary: {}"}}"#, e);
            }
        };

        // Encode to base64
        let encoded = match encode_to_base64(&info) {
            Ok(encoded) => {
                debug!(encoded_length = encoded.len(), "Successfully encoded itinerary to base64");
                encoded
            },
            Err(e) => {
                error!("Error encoding itinerary: {}", e);
                return format!(r#"{{"error": "Error encoding itinerary: {}"}}"#, e);
            }
        };

        // Format as Google Flights URL
        let url = format!("https://www.google.com/travel/flights?tfs={}", encoded);
        
        info!(url = url, "Successfully generated itinerary link");
        
        serde_json::json!({
            "url": url,
            "message": "Generated Google Flights itinerary link for selected flights"
        }).to_string()
    }
}

// Helper functions for parameter conversion
fn build_flight_search_request(
    from_airport: String,
    to_airport: String,
    params: FlightSearchParams,
) -> Result<FlightSearchRequest, String> {
    let passengers = Passengers {
        adults: params.adults.unwrap_or(1),
        children: params.children.unwrap_or(0),
        infants_in_seat: params.infants_in_seat.unwrap_or(0),
        infants_on_lap: params.infants_on_lap.unwrap_or(0),
    };

    let trip_type = params
        .trip_type
        .as_deref()
        .unwrap_or("one-way")
        .parse::<TripType>()
        .map_err(|e| format!("Invalid trip type: {}", e))?;

    let seat_class = params
        .seat_class
        .as_deref()
        .unwrap_or("economy")
        .parse::<SeatClass>()
        .map_err(|e| format!("Invalid seat class: {}", e))?;

    let departure_time = params
        .departure_time
        .as_deref()
        .map(TimeWindow::from_range_str)
        .transpose()
        .map_err(|e| format!("Invalid departure time: {}", e))?;

    let arrival_time = params
        .arrival_time
        .as_deref()
        .map(TimeWindow::from_range_str)
        .transpose()
        .map_err(|e| format!("Invalid arrival time: {}", e))?;

    let max_stops = params.max_stops;
    let return_date = params.return_date;

    // Parse comma-delimited airlines string
    let parsed_airlines = params.airlines.as_ref().map(|airlines_str| {
        airlines_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
    });

    let mut flights = vec![FlightData {
        date: params.departure_date,
        from_airport: from_airport.clone(),
        to_airport: to_airport.clone(),
        max_stops,
        airlines: parsed_airlines.clone(),
        departure_time,
        arrival_time: arrival_time.clone(),
    }];

    if let (TripType::RoundTrip, Some(return_date)) = (&trip_type, return_date) {
        flights.push(FlightData {
            date: return_date,
            from_airport: to_airport,
            to_airport: from_airport,
            max_stops,
            airlines: parsed_airlines,
            departure_time: None, // Times usually not specified for return
            arrival_time,
        });
    }

    Ok(FlightSearchRequest {
        flights,
        trip_type,
        passengers,
        seat_class,
    })
}

fn build_city_flight_search_request(
    from_city: String,
    to_city: String,
    params: FlightSearchParams,
) -> Result<CityFlightSearchRequest, String> {
    let passengers = Passengers {
        adults: params.adults.unwrap_or(1),
        children: params.children.unwrap_or(0),
        infants_in_seat: params.infants_in_seat.unwrap_or(0),
        infants_on_lap: params.infants_on_lap.unwrap_or(0),
    };

    let trip_type = params
        .trip_type
        .as_deref()
        .unwrap_or("one-way")
        .parse::<TripType>()
        .map_err(|e| format!("Invalid trip type: {}", e))?;

    let seat_class = params
        .seat_class
        .as_deref()
        .unwrap_or("economy")
        .parse::<SeatClass>()
        .map_err(|e| format!("Invalid seat class: {}", e))?;

    let departure_time = params
        .departure_time
        .as_deref()
        .map(TimeWindow::from_range_str)
        .transpose()
        .map_err(|e| format!("Invalid departure time: {}", e))?;

    let arrival_time = params
        .arrival_time
        .as_deref()
        .map(TimeWindow::from_range_str)
        .transpose()
        .map_err(|e| format!("Invalid arrival time: {}", e))?;

    let max_stops = params.max_stops;
    let return_date = params.return_date;

    // Parse comma-delimited airlines string
    let parsed_airlines = params.airlines.as_ref().map(|airlines_str| {
        airlines_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
    });

    let mut flights = vec![CityFlightData {
        date: params.departure_date,
        from_city: from_city.clone(),
        to_city: to_city.clone(),
        max_stops,
        airlines: parsed_airlines.clone(),
        departure_time,
        arrival_time: arrival_time.clone(),
    }];

    if let (TripType::RoundTrip, Some(return_date)) = (&trip_type, return_date) {
        flights.push(CityFlightData {
            date: return_date,
            from_city: to_city,
            to_city: from_city,
            max_stops,
            airlines: parsed_airlines,
            departure_time: None, // Times usually not specified for return
            arrival_time,
        });
    }

    Ok(CityFlightSearchRequest {
        flights,
        trip_type,
        passengers,
        seat_class,
    })
}

fn format_flight_results_json(result: FlightResult, max_flights: Option<usize>) -> String {
    if result.flights.is_empty() {
        return serde_json::json!({
            "total_flights": 0,
            "current_price_level": result.current_price,
            "best_flights": [],
            "other_flights": [],
            "message": "No flights found matching your criteria."
        }).to_string();
    }

    let mut best_flights = Vec::new();
    let mut other_flights = Vec::new();
    let limit = max_flights.unwrap_or(30);
    
    // Take only the requested number of flights
    let flights_to_process = result.flights.into_iter().take(limit);

    for flight in flights_to_process {
        let flight_info = FlightInfo {
            airline_name: flight.name,
            departure_time: flight.departure,
            arrival_time: flight.arrival,
            duration: flight.duration,
            origin: flight.origin,
            destination: flight.destination,
            stops: flight.stops,
            price: flight.price,
            airline_code: flight.airline_code,
            flight_number: flight.flight_number,
        };

        if flight.is_best {
            best_flights.push(flight_info);
        } else {
            other_flights.push(flight_info);
        }
    }

    let total_flights = best_flights.len() + other_flights.len();
    let search_result = FlightSearchResult {
        total_flights,
        current_price_level: result.current_price,
        best_flights,
        other_flights,
    };

    serde_json::to_string_pretty(&search_result).unwrap_or_else(|e| {
        format!(r#"{{"error": "Failed to serialize results: {}"}}"#, e)
    })
}

#[tool(tool_box)]
impl ServerHandler for FlightServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A flight search server with unified airport and city search capabilities. Returns structured JSON results with best_flights and other_flights. Also provides itinerary link generation for selected flights.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging before anything else
    if let Err(e) = FlightServer::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        // Continue without logging rather than failing
    }

    info!("Starting MCP Flight Server");
    debug!("Debug logging test from main function");
    
    let server = FlightServer::new();
    let transport = stdio();

    info!("MCP server initialized, starting service");
    debug!("About to start MCP service");

    // SDK handles initialization, tool discovery, and message routing
    let service = server.serve(transport).await?;

    info!("MCP service started, waiting for requests");

    // Wait for shutdown
    service.waiting().await?;

    info!("MCP service shutting down");
    Ok(())
} 
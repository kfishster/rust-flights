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
    Passengers, SeatClass, TimeWindow, TripType,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Flight search MCP server
#[derive(Default, Clone)]
pub struct FlightServer;

impl FlightServer {
    pub fn new() -> Self {
        Self
    }
}

/// Unified flight search parameters with explicit mode selection
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FlightSearchParams {
    /// Airport-based search parameters (mutually exclusive with cities)
    #[schemars(description = "Airport search with from/to airport codes")]
    pub airports: Option<AirportSearch>,
    /// City-based search parameters (mutually exclusive with airports)
    #[schemars(description = "City search with from/to city names")]
    pub cities: Option<CitySearch>,
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
    #[schemars(description = "Preferred airlines")]
    pub airlines: Option<Vec<String>>,
    #[schemars(description = "Departure time window in HH:MM-HH:MM format")]
    pub departure_time: Option<String>,
    #[schemars(description = "Arrival time window in HH:MM-HH:MM format")]
    pub arrival_time: Option<String>,
    #[schemars(description = "Trip type: one-way or round-trip")]
    pub trip_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
pub struct AirportSearch {
    #[schemars(description = "Origin airport code (e.g., LAX, JFK)")]
    pub from_airport: String,
    #[schemars(description = "Destination airport code (e.g., JFK, LHR)")]
    pub to_airport: String,
}

#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
pub struct CitySearch {
    #[schemars(description = "Origin city name (e.g., Los Angeles, New York)")]
    pub from_city: String,
    #[schemars(description = "Destination city name (e.g., New York, London)")]
    pub to_city: String,
}

#[derive(Debug, Serialize)]
pub struct FlightSearchResult {
    pub total_flights: usize,
    pub current_price_level: String,
    pub best_flights: Vec<FlightInfo>,
    pub other_flights: Vec<FlightInfo>,
}

#[derive(Debug, Serialize)]
pub struct FlightInfo {
    pub is_best: bool,
    pub airline_name: String,
    pub departure_time: String,
    pub arrival_time: String,
    pub arrival_time_ahead: String,
    pub duration: String,
    pub stops: i32,
    pub stops_description: String,
    pub price: String,
    pub delay: Option<String>,
}

#[tool(tool_box)]
impl FlightServer {
    /// Unified flight search with explicit mode selection
    #[tool(description = "Search for flights between locations. Specify either 'airports' for airport code search or 'cities' for city name search.")]
    async fn get_flights(
        &self,
        #[tool(aggr)] params: FlightSearchParams,
    ) -> String {
        let result = match (params.airports.as_ref(), params.cities.as_ref()) {
            (Some(airports), None) => {
                match build_flight_search_request(airports.clone(), params) {
                    Ok(request) => get_flights_internal(request).await,
                    Err(e) => return format!(r#"{{"error": "Error building flight request: {}"}}"#, e),
                }
            }
            (None, Some(cities)) => {
                match build_city_flight_search_request(cities.clone(), params) {
                    Ok(request) => get_flights_by_city_internal(request).await,
                    Err(e) => return format!(r#"{{"error": "Error building city flight request: {}"}}"#, e),
                }
            }
            (None, None) => {
                return r#"{"error": "Must specify either 'airports' or 'cities' for flight search"}"#.to_string()
            }
            (Some(_), Some(_)) => {
                return r#"{"error": "Cannot specify both 'airports' and 'cities' - choose one search mode"}"#.to_string()
            }
        };

        match result {
            Ok(flight_result) => format_flight_results_json(flight_result),
            Err(e) => format!(r#"{{"error": "Flight search failed: {}"}}"#, e),
        }
    }
}

// Helper functions for parameter conversion
fn build_flight_search_request(
    airports: AirportSearch,
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

    let airlines = params.airlines.clone();
    let max_stops = params.max_stops;
    let return_date = params.return_date;

    let mut flights = vec![FlightData {
        date: params.departure_date,
        from_airport: airports.from_airport,
        to_airport: airports.to_airport,
        max_stops,
        airlines: airlines.clone(),
        departure_time,
        arrival_time: arrival_time.clone(),
    }];

    if let (TripType::RoundTrip, Some(return_date)) = (&trip_type, return_date) {
        flights.push(FlightData {
            date: return_date,
            from_airport: flights[0].to_airport.clone(),
            to_airport: flights[0].from_airport.clone(),
            max_stops,
            airlines,
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
    cities: CitySearch,
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

    let airlines = params.airlines.clone();
    let max_stops = params.max_stops;
    let return_date = params.return_date;

    let mut flights = vec![CityFlightData {
        date: params.departure_date,
        from_city: cities.from_city,
        to_city: cities.to_city,
        max_stops,
        airlines: airlines.clone(),
        departure_time,
        arrival_time: arrival_time.clone(),
    }];

    if let (TripType::RoundTrip, Some(return_date)) = (&trip_type, return_date) {
        flights.push(CityFlightData {
            date: return_date,
            from_city: flights[0].to_city.clone(),
            to_city: flights[0].from_city.clone(),
            max_stops,
            airlines,
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

fn format_flight_results_json(result: FlightResult) -> String {
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
    let total_flights = result.flights.len();

    for flight in result.flights {
        let stops_description = match flight.stops {
            0 => "Nonstop".to_string(),
            1 => "1 stop".to_string(),
            n => format!("{} stops", n),
        };

        let flight_info = FlightInfo {
            is_best: flight.is_best,
            airline_name: flight.name,
            departure_time: flight.departure,
            arrival_time: flight.arrival,
            arrival_time_ahead: flight.arrival_time_ahead,
            duration: flight.duration,
            stops: flight.stops,
            stops_description,
            price: flight.price,
            delay: flight.delay,
        };

        if flight.is_best {
            best_flights.push(flight_info);
        } else {
            other_flights.push(flight_info);
        }
    }

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
            instructions: Some("A flight search server with unified airport and city search capabilities. Returns structured JSON results with best_flights and other_flights.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = FlightServer::new();
    let transport = stdio();

    // SDK handles initialization, tool discovery, and message routing
    let service = server.serve(transport).await?;

    // Wait for shutdown
    service.waiting().await?;

    Ok(())
} 
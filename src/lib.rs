//! # Rust Flights Library
//! 
//! A high-performance Rust library that reverse engineers the Google Flights API.
//! This library provides better performance than the existing Python implementation
//! while maintaining API compatibility.

pub mod client;
pub mod protobuf;
pub mod wikidata;

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

// Re-export main types for convenience
pub use client::{FlightClient, FlightResponseParser};
pub use protobuf::*;
pub use wikidata::{WikidataClient, CityInfo, WikidataError};

/// Error types for the flights library
#[derive(Error, Debug)]
pub enum FlightError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("HTML parsing failed: {0}")]
    ParseError(String),
    
    #[error("Protobuf encoding failed: {0}")]
    ProtobufError(#[from] prost::EncodeError),
    
    #[error("City not found: {0}")]
    CityNotFound(String),
    
    #[error("Invalid date format: {0}")]
    DateParseError(String),
    
    #[error("Wikidata API error: {0}")]
    WikidataApiError(#[from] WikidataError),
    
    #[error("Invalid time format: {0}")]
    TimeParseError(String),
}

/// Time window for departure or arrival filtering
#[derive(Debug, Clone, PartialEq)]
pub struct TimeWindow {
    pub earliest_hour: i32,  // 0-23 (24-hour format)
    pub latest_hour: i32,    // 0-23 (24-hour format)
}

impl TimeWindow {
    /// Create a new TimeWindow
    pub fn new(earliest_hour: i32, latest_hour: i32) -> Result<Self, FlightError> {
        if earliest_hour < 0 || earliest_hour > 23 {
            return Err(FlightError::TimeParseError(
                format!("earliest_hour must be 0-23, got {}", earliest_hour)
            ));
        }
        if latest_hour < 0 || latest_hour > 23 {
            return Err(FlightError::TimeParseError(
                format!("latest_hour must be 0-23, got {}", latest_hour)
            ));
        }
        
        Ok(Self {
            earliest_hour,
            latest_hour,
        })
    }
    
    /// Parse from HH:MM-HH:MM format (e.g., "06:00-11:00")
    pub fn from_range_str(range: &str) -> Result<Self, FlightError> {
        let parts: Vec<&str> = range.split('-').collect();
        if parts.len() != 2 {
            return Err(FlightError::TimeParseError(
                format!("Time range must be in format HH:MM-HH:MM, got {}", range)
            ));
        }
        
        let earliest_hour = Self::parse_hour(parts[0])?;
        let latest_hour = Self::parse_hour(parts[1])?;
        
        Self::new(earliest_hour, latest_hour)
    }
    
    fn parse_hour(time_str: &str) -> Result<i32, FlightError> {
        let hour_str = time_str.split(':').next().unwrap_or("");
        hour_str.parse::<i32>().map_err(|_| {
            FlightError::TimeParseError(
                format!("Invalid hour format: {}", time_str)
            )
        })
    }
}

/// Core flight data structure matching Python implementation
#[derive(Debug, Clone)]
pub struct FlightData {
    pub date: String,
    pub from_airport: String,      // Airport code
    pub to_airport: String,        // Airport code
    pub max_stops: Option<i32>,
    pub airlines: Option<Vec<String>>,
    pub departure_time: Option<TimeWindow>,
    pub arrival_time: Option<TimeWindow>,
}

/// Complete flight search request with all parameters
#[derive(Debug, Clone)]
pub struct FlightSearchRequest {
    pub flights: Vec<FlightData>,
    pub trip_type: TripType,
    pub passengers: Passengers,
    pub seat_class: SeatClass,
}

/// Passenger configuration
#[derive(Debug, Clone)]
pub struct Passengers {
    pub adults: i32,
    pub children: i32,
    pub infants_in_seat: i32,
    pub infants_on_lap: i32,
}

impl Default for Passengers {
    fn default() -> Self {
        Self {
            adults: 1,
            children: 0,
            infants_in_seat: 0,
            infants_on_lap: 0,
        }
    }
}

/// Flight search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightResult {
    pub current_price: String,      // "low", "typical", "high"
    pub flights: Vec<Flight>,
}

/// Individual flight information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub is_best: bool,
    pub name: String,
    pub departure: String,
    pub arrival: String,
    pub duration: String,
    pub stops: i32,
    pub price: FlightPrice,
    pub airline_code: Option<String>,
    pub flight_number: Option<String>,
    pub origin_airport: Option<String>,
    pub destination_airport: Option<String>,
}

/// Price information with amount and currency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightPrice {
    pub amount: i32,
    pub currency: String,
}

/// Trip type enumeration
#[derive(Debug, Clone)]
pub enum TripType {
    RoundTrip,
    OneWay,
    MultiCity,
}

impl FromStr for TripType {
    type Err = FlightError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "round-trip" | "roundtrip" => Ok(TripType::RoundTrip),
            "one-way" | "oneway" => Ok(TripType::OneWay),
            "multi-city" | "multicity" => Ok(TripType::MultiCity),
            _ => Err(FlightError::ParseError(format!("Invalid trip type: {}", s))),
        }
    }
}

/// Seat class enumeration
#[derive(Debug, Clone)]
pub enum SeatClass {
    Economy,
    PremiumEconomy,
    Business,
    First,
}

impl FromStr for SeatClass {
    type Err = FlightError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "economy" => Ok(SeatClass::Economy),
            "premium-economy" | "premium_economy" => Ok(SeatClass::PremiumEconomy),
            "business" => Ok(SeatClass::Business),
            "first" => Ok(SeatClass::First),
            _ => Err(FlightError::ParseError(format!("Invalid seat class: {}", s))),
        }
    }
}

/// Main public API function with consolidated parameters
pub async fn get_flights(request: FlightSearchRequest) -> Result<FlightResult, FlightError> {
    let client = FlightClient::new().await?;
    client.get_flights(request).await
}

/// Legacy API function matching Python interface (deprecated)
#[deprecated(since = "0.1.0", note = "Use get_flights(FlightSearchRequest) instead")]
pub async fn get_flights_legacy(
    flight_data: Vec<FlightData>,
    trip: &str,
    passengers: Passengers,
    seat: &str,
    max_stops: Option<i32>,
) -> Result<FlightResult, FlightError> {
    let trip_type = trip.parse::<TripType>()?;
    let seat_class = seat.parse::<SeatClass>()?;
    
    // Apply max_stops to all flights that don't have it set
    let mut flights = flight_data;
    for flight in &mut flights {
        if flight.max_stops.is_none() {
            flight.max_stops = max_stops;
        }
    }
    
    let request = FlightSearchRequest {
        flights,
        trip_type,
        passengers,
        seat_class,
    };
    
    get_flights(request).await
}

/// City-based flight data structure (Phase 4)
#[derive(Debug, Clone)]
pub struct CityFlightData {
    pub date: String,
    pub from_city: String,      // City name (e.g., "London", "New York")
    pub to_city: String,        // City name (e.g., "Paris", "Tokyo")
    pub max_stops: Option<i32>,
    pub airlines: Option<Vec<String>>,
    pub departure_time: Option<TimeWindow>,
    pub arrival_time: Option<TimeWindow>,
}

/// City-based flight search request
#[derive(Debug, Clone)]
pub struct CityFlightSearchRequest {
    pub flights: Vec<CityFlightData>,
    pub trip_type: TripType,
    pub passengers: Passengers,
    pub seat_class: SeatClass,
}

/// **Phase 4: NEW CITY-BASED API**
/// Search flights using city names instead of airport codes.
/// This function automatically resolves city names to Freebase IDs using Wikidata.
/// 
/// # Example
/// ```rust
/// use rust_flights::{get_flights_by_city, CityFlightSearchRequest, CityFlightData, TripType, SeatClass, Passengers};
/// 
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let request = CityFlightSearchRequest {
///     flights: vec![CityFlightData {
///         date: "2025-08-15".to_string(),
///         from_city: "London".to_string(),
///         to_city: "New York".to_string(),
///         max_stops: Some(1),
///         airlines: None,
///         departure_time: None,
///         arrival_time: None,
///     }],
///     trip_type: TripType::OneWay,
///     passengers: Passengers::default(),
///     seat_class: SeatClass::Economy,
/// };
/// 
/// let result = get_flights_by_city(request).await?;
/// println!("Found {} flights", result.flights.len());
/// # Ok(())
/// # }
/// ```
pub async fn get_flights_by_city(request: CityFlightSearchRequest) -> Result<FlightResult, FlightError> {
    let wikidata_client = WikidataClient::new()?;
    
    // Convert city names to Freebase IDs
    let mut airport_flights = Vec::new();
    
    for city_flight in request.flights {
        // Resolve city names to Freebase IDs
        let from_freebase_id = wikidata_client
            .get_freebase_id_only(&city_flight.from_city)
            .await
            .map_err(|_| FlightError::CityNotFound(city_flight.from_city.clone()))?;
            
        let to_freebase_id = wikidata_client
            .get_freebase_id_only(&city_flight.to_city)
            .await
            .map_err(|_| FlightError::CityNotFound(city_flight.to_city.clone()))?;
        
        // Create FlightData with Freebase IDs instead of airport codes
        let flight_data = FlightData {
            date: city_flight.date,
            from_airport: from_freebase_id,  // Use Freebase ID instead of airport code
            to_airport: to_freebase_id,      // Use Freebase ID instead of airport code
            max_stops: city_flight.max_stops,
            airlines: city_flight.airlines,
            departure_time: city_flight.departure_time,
            arrival_time: city_flight.arrival_time,
        };
        
        airport_flights.push(flight_data);
    }
    
    // Create regular flight search request with Freebase IDs
    let airport_request = FlightSearchRequest {
        flights: airport_flights,
        trip_type: request.trip_type,
        passengers: request.passengers,
        seat_class: request.seat_class,
    };
    
    // Use existing flight search API
    get_flights(airport_request).await
}

/// **Phase 4: CONVENIENCE FUNCTION**
/// Simple one-way city-based flight search with minimal parameters.
/// 
/// # Example
/// ```rust
/// use rust_flights::search_flights_between_cities;
/// 
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = search_flights_between_cities(
///     "London", 
///     "Paris", 
///     "2025-08-15"
/// ).await?;
/// 
/// println!("Found {} flights from London to Paris", result.flights.len());
/// # Ok(())
/// # }
/// ```
pub async fn search_flights_between_cities(
    from_city: &str,
    to_city: &str,
    date: &str,
) -> Result<FlightResult, FlightError> {
    let request = CityFlightSearchRequest {
        flights: vec![CityFlightData {
            date: date.to_string(),
            from_city: from_city.to_string(),
            to_city: to_city.to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: None,
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    get_flights_by_city(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_window_creation() {
        let window = TimeWindow::new(9, 17).unwrap();
        assert_eq!(window.earliest_hour, 9);
        assert_eq!(window.latest_hour, 17);
        
        // Test invalid hours
        assert!(TimeWindow::new(-1, 10).is_err());
        assert!(TimeWindow::new(10, 24).is_err());
    }
    
    #[test]
    fn test_time_window_from_range_str() {
        let window = TimeWindow::from_range_str("09:00-17:00").unwrap();
        assert_eq!(window.earliest_hour, 9);
        assert_eq!(window.latest_hour, 17);
        
        let window = TimeWindow::from_range_str("00:00-11:00").unwrap();
        assert_eq!(window.earliest_hour, 0);
        assert_eq!(window.latest_hour, 11);
        
        // Test invalid formats
        assert!(TimeWindow::from_range_str("09:00").is_err());
        assert!(TimeWindow::from_range_str("invalid-time").is_err());
    }

    #[test]
    fn test_trip_type_parsing() {
        assert!(matches!("round-trip".parse::<TripType>(), Ok(TripType::RoundTrip)));
        assert!(matches!("one-way".parse::<TripType>(), Ok(TripType::OneWay)));
        assert!(matches!("multi-city".parse::<TripType>(), Ok(TripType::MultiCity)));
        assert!("invalid".parse::<TripType>().is_err());
    }

    #[test]
    fn test_seat_class_parsing() {
        assert!(matches!("economy".parse::<SeatClass>(), Ok(SeatClass::Economy)));
        assert!(matches!("premium-economy".parse::<SeatClass>(), Ok(SeatClass::PremiumEconomy)));
        assert!(matches!("business".parse::<SeatClass>(), Ok(SeatClass::Business)));
        assert!(matches!("first".parse::<SeatClass>(), Ok(SeatClass::First)));
        assert!("invalid".parse::<SeatClass>().is_err());
    }

    #[test]
    fn test_passengers_default() {
        let passengers = Passengers::default();
        assert_eq!(passengers.adults, 1);
        assert_eq!(passengers.children, 0);
        assert_eq!(passengers.infants_in_seat, 0);
        assert_eq!(passengers.infants_on_lap, 0);
    }
} 
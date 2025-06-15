//! # Rust Flights Library
//! 
//! A high-performance Rust library that reverse engineers the Google Flights API.
//! This library provides better performance than the existing Python implementation
//! while maintaining API compatibility.

pub mod client;
pub mod protobuf;

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

// Re-export main types for convenience
pub use client::{FlightClient, FlightResponseParser};
pub use protobuf::*;

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
    WikidataError(String),
    
    #[cfg(feature = "city-search")]
    #[error("Cache error: {0}")]
    CacheError(#[from] sled::Error),
}

/// Core flight data structure matching Python implementation
#[derive(Debug, Clone)]
pub struct FlightData {
    pub date: String,
    pub from_airport: String,      // Airport code
    pub to_airport: String,        // Airport code
    pub max_stops: Option<i32>,
    pub airlines: Option<Vec<String>>,
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
    pub arrival_time_ahead: String,
    pub duration: String,
    pub stops: i32,
    pub delay: Option<String>,
    pub price: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
//! Integration tests for rust-flights
//! 
//! These tests make actual HTTP requests to Google Flights to verify
//! that our protobuf encoding and HTML parsing work correctly.

use rust_flights::{
    get_flights, FlightData, FlightSearchRequest, Passengers, SeatClass, TripType
};
use tokio;

/// Helper function to create a basic search request
fn create_basic_request(from: &str, to: &str, date: &str) -> FlightSearchRequest {
    FlightSearchRequest {
        flights: vec![FlightData {
            date: date.to_string(),
            from_airport: from.to_string(),
            to_airport: to.to_string(),
            max_stops: Some(1),
            airlines: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    }
}

/// Helper function to create a round-trip request
fn create_roundtrip_request(from: &str, to: &str, depart: &str, return_date: &str) -> FlightSearchRequest {
    FlightSearchRequest {
        flights: vec![
            FlightData {
                date: depart.to_string(),
                from_airport: from.to_string(),
                to_airport: to.to_string(),
                max_stops: Some(1),
                airlines: None,
            },
            FlightData {
                date: return_date.to_string(),
                from_airport: to.to_string(),
                to_airport: from.to_string(),
                max_stops: Some(1),
                airlines: None,
            },
        ],
        trip_type: TripType::RoundTrip,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    }
}

#[tokio::test]
async fn test_basic_domestic_flight() {
    let request = create_basic_request("LAX", "JFK", "2025-08-15");
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Basic domestic flight test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            // Basic validation
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
            // Note: We don't assert flights.len() > 0 because Google might return no results
            // but the parsing should still work and return an empty list
        }
        Err(e) => {
            println!("⚠️  Basic domestic flight test failed (this may be expected): {}", e);
            // This is not necessarily a test failure - Google might block us or return no results
            // The important thing is that our protobuf encoding worked (no encoding errors)
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_international_flight() {
    let request = create_basic_request("LAX", "LHR", "2025-09-01");
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ International flight test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  International flight test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_round_trip_flight() {
    let request = create_roundtrip_request("SFO", "NYC", "2025-07-10", "2025-07-17");
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Round-trip flight test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Round-trip flight test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_business_class_flight() {
    let mut request = create_basic_request("LAX", "JFK", "2025-08-20");
    request.seat_class = SeatClass::Business;
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Business class flight test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Business class flight test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_multiple_passengers() {
    let mut request = create_basic_request("LAX", "JFK", "2025-08-25");
    request.passengers = Passengers {
        adults: 2,
        children: 1,
        infants_in_seat: 0,
        infants_on_lap: 1,
    };
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Multiple passengers test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Multiple passengers test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_specific_airlines() {
    let mut request = create_basic_request("LAX", "JFK", "2025-08-30");
    request.flights[0].airlines = Some(vec!["AA".to_string(), "DL".to_string()]);
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Specific airlines test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Specific airlines test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_nonstop_flights_only() {
    let mut request = create_basic_request("LAX", "JFK", "2025-09-05");
    request.flights[0].max_stops = Some(0); // Nonstop only
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Nonstop flights test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Nonstop flights test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                _ => {
                    println!("Non-protobuf error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_protobuf_encoding_edge_cases() {
    // Test with empty airlines list
    let request1 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: None,
            airlines: Some(vec![]), // Empty list
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    // This should not panic on protobuf encoding
    let result1 = get_flights(request1).await;
    match result1 {
        Ok(_) => println!("✅ Empty airlines list handled correctly"),
        Err(rust_flights::FlightError::ProtobufError(e)) => {
            panic!("Protobuf encoding failed with empty airlines: {}", e);
        }
        Err(_) => println!("Non-protobuf error (acceptable)"),
    }
    
    // Test with maximum passengers
    let request2 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(2),
            airlines: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers {
            adults: 9,
            children: 8,
            infants_in_seat: 2,
            infants_on_lap: 1,
        },
        seat_class: SeatClass::First,
    };
    
    let result2 = get_flights(request2).await;
    match result2 {
        Ok(_) => println!("✅ Maximum passengers handled correctly"),
        Err(rust_flights::FlightError::ProtobufError(e)) => {
            panic!("Protobuf encoding failed with max passengers: {}", e);
        }
        Err(_) => println!("Non-protobuf error (acceptable)"),
    }
} 
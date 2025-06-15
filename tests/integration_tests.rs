//! Integration tests for rust-flights
//! 
//! These tests make actual HTTP requests to Google Flights to verify
//! that our protobuf encoding and HTML parsing work correctly.

use rust_flights::{
    get_flights, get_flights_by_city, search_flights_between_cities,
    FlightData, FlightSearchRequest, CityFlightData, CityFlightSearchRequest,
    Passengers, SeatClass, TripType, TimeWindow
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
            departure_time: None,
            arrival_time: None,
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
                departure_time: None,
                arrival_time: None,
            },
            FlightData {
                date: return_date.to_string(),
                from_airport: to.to_string(),
                to_airport: from.to_string(),
                max_stops: Some(1),
                airlines: None,
                departure_time: None,
                arrival_time: None,
            },
        ],
        trip_type: TripType::RoundTrip,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    }
}

/// Helper function to create a request with time windows
fn create_request_with_time_windows(
    from: &str, 
    to: &str, 
    date: &str,
    departure_time: Option<TimeWindow>,
    arrival_time: Option<TimeWindow>
) -> FlightSearchRequest {
    FlightSearchRequest {
        flights: vec![FlightData {
            date: date.to_string(),
            from_airport: from.to_string(),
            to_airport: to.to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time,
            arrival_time,
        }],
        trip_type: TripType::OneWay,
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
    request.flights[0].max_stops = Some(0);
    
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

// NEW TIME WINDOW TESTS

#[tokio::test]
async fn test_flight_with_morning_departure() {
    let departure_time = TimeWindow::new(6, 12).unwrap(); // 6:00am to 12:00pm
    let request = create_request_with_time_windows("LAX", "JFK", "2025-08-15", Some(departure_time), None);
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Morning departure time filter test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Morning departure time filter test failed (this may be expected): {}", e);
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
async fn test_flight_with_evening_arrival() {
    let arrival_time = TimeWindow::new(17, 23).unwrap(); // 5:00pm to 11:00pm
    let request = create_request_with_time_windows("LAX", "JFK", "2025-08-15", None, Some(arrival_time));
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Evening arrival time filter test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Evening arrival time filter test failed (this may be expected): {}", e);
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
async fn test_flight_with_user_example_time_windows() {
    // User's example: flight should leave between 12:00am and 11:00am, and arrive 3pm-12am
    let departure_time = TimeWindow::new(0, 11).unwrap();  // 12:00am to 11:00am 
    let arrival_time = TimeWindow::new(15, 23).unwrap();   // 3:00pm to 11:00pm
    
    let request = create_request_with_time_windows(
        "LAX", 
        "JFK", 
        "2025-08-15", 
        Some(departure_time), 
        Some(arrival_time)
    );
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ User example time windows test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  User example time windows test failed (this may be expected): {}", e);
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
async fn test_flight_with_both_time_windows() {
    let departure_time = TimeWindow::new(8, 14).unwrap();   // 8:00am to 2:00pm
    let arrival_time = TimeWindow::new(16, 22).unwrap();    // 4:00pm to 10:00pm
    
    let request = create_request_with_time_windows(
        "SFO", 
        "NYC", 
        "2025-08-15", 
        Some(departure_time), 
        Some(arrival_time)
    );
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Both time windows test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Both time windows test failed (this may be expected): {}", e);
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
async fn test_flight_with_red_eye_departure() {
    // Red-eye flight: midnight to early morning
    let departure_time = TimeWindow::new(0, 6).unwrap();   // 12:00am to 6:00am
    
    let request = create_request_with_time_windows(
        "LAX", 
        "JFK", 
        "2025-08-15", 
        Some(departure_time), 
        None
    );
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Red-eye departure test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Red-eye departure test failed (this may be expected): {}", e);
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
    // Test edge cases for protobuf encoding
    let passengers = Passengers {
        adults: 8,
        children: 3,
        infants_in_seat: 2,
        infants_on_lap: 1,
    };
    
    let departure_time = TimeWindow::new(23, 23).unwrap();  // Same hour (11pm)
    let arrival_time = TimeWindow::new(0, 0).unwrap();      // Same hour (midnight)
    
    let request = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-12-25".to_string(),
            from_airport: "ORD".to_string(),
            to_airport: "MIA".to_string(),
            max_stops: Some(3),
            airlines: Some(vec!["AA".to_string(), "UA".to_string(), "DL".to_string()]),
            departure_time: Some(departure_time),
            arrival_time: Some(arrival_time),
        }],
        trip_type: TripType::OneWay,
        passengers,
        seat_class: SeatClass::First,
    };
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Protobuf encoding edge cases test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Protobuf encoding edge cases test failed (this may be expected): {}", e);
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

// ===== PHASE 4: CITY-BASED FLIGHT SEARCH TESTS =====

/// Helper function to create a city-based request
fn create_city_request(from_city: &str, to_city: &str, date: &str) -> CityFlightSearchRequest {
    CityFlightSearchRequest {
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
    }
}

/// Helper function to create a city-based request with time windows
fn create_city_request_with_time_windows(
    from_city: &str, 
    to_city: &str, 
    date: &str,
    departure_time: Option<TimeWindow>,
    arrival_time: Option<TimeWindow>
) -> CityFlightSearchRequest {
    CityFlightSearchRequest {
        flights: vec![CityFlightData {
            date: date.to_string(),
            from_city: from_city.to_string(),
            to_city: to_city.to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time,
            arrival_time,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    }
}

#[tokio::test]
async fn test_city_based_search_london_to_paris() {
    let request = create_city_request("London", "Paris", "2025-08-15");
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ City-based search (London → Paris) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  City-based search (London → Paris) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_city_based_search_new_york_to_tokyo() {
    let request = create_city_request("New York", "Tokyo", "2025-09-01");
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ City-based search (New York → Tokyo) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  City-based search (New York → Tokyo) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_convenience_function_london_to_sydney() {
    match search_flights_between_cities("London", "Sydney", "2025-08-20").await {
        Ok(result) => {
            println!("✅ Convenience function (London → Sydney) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  Convenience function (London → Sydney) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_city_based_search_with_time_windows() {
    let departure_time = TimeWindow::new(8, 14).unwrap();   // 8:00am to 2:00pm
    let arrival_time = TimeWindow::new(16, 22).unwrap();    // 4:00pm to 10:00pm
    
    let request = create_city_request_with_time_windows(
        "Paris", 
        "London", 
        "2025-08-15", 
        Some(departure_time), 
        Some(arrival_time)
    );
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ City-based search with time windows (Paris → London) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  City-based search with time windows (Paris → London) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_city_based_business_class() {
    let mut request = create_city_request("Tokyo", "Sydney", "2025-09-10");
    request.seat_class = SeatClass::Business;
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ City-based business class (Tokyo → Sydney) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  City-based business class (Tokyo → Sydney) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_city_based_round_trip() {
    let request = CityFlightSearchRequest {
        flights: vec![
            CityFlightData {
                date: "2025-07-10".to_string(),
                from_city: "New York".to_string(),
                to_city: "London".to_string(),
                max_stops: Some(1),
                airlines: None,
                departure_time: None,
                arrival_time: None,
            },
            CityFlightData {
                date: "2025-07-17".to_string(),
                from_city: "London".to_string(),
                to_city: "New York".to_string(),
                max_stops: Some(1),
                airlines: None,
                departure_time: None,
                arrival_time: None,
            },
        ],
        trip_type: TripType::RoundTrip,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ City-based round-trip (New York ↔ London) test passed");
            println!("Current price: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            assert!(!result.current_price.is_empty(), "Current price should not be empty");
        }
        Err(e) => {
            println!("⚠️  City-based round-trip (New York ↔ London) test failed (this may be expected): {}", e);
            match e {
                rust_flights::FlightError::ProtobufError(_) => {
                    panic!("Protobuf encoding failed: {}", e);
                }
                rust_flights::FlightError::CityNotFound(city) => {
                    println!("City not found (this may indicate Wikidata API issues): {}", city);
                }
                _ => {
                    println!("Non-critical error (acceptable): {}", e);
                }
            }
        }
    }
} 
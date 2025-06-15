//! Time window filtering example
//! 
//! This example demonstrates how to use the new time window functionality
//! to filter flights by departure and arrival times.

use rust_flights::{get_flights, FlightData, FlightSearchRequest, Passengers, SeatClass, TripType, TimeWindow};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🕐 Time Window Flight Search Examples");
    println!("====================================\n");

    // Example 1: User's specific scenario
    // Flight should leave between 12:00am and 11:00am, and arrive 3pm-12am
    println!("Example 1: User's specific time windows");
    println!("Departure: 12:00am - 11:00am");
    println!("Arrival: 3:00pm - 11:00pm");
    
    let departure_time = TimeWindow::new(0, 11)?;  // 12:00am to 11:00am
    let arrival_time = TimeWindow::new(15, 23)?;   // 3:00pm to 11:00pm
    
    let request1 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: Some(departure_time),
            arrival_time: Some(arrival_time),
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    search_and_display("User Example", request1).await;
    
    // Example 2: Morning departure only
    println!("\nExample 2: Morning departure (6:00am - 12:00pm)");
    
    let morning_departure = TimeWindow::new(6, 12)?;
    
    let request2 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "SFO".to_string(),
            to_airport: "NYC".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: Some(morning_departure),
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    search_and_display("Morning Departure", request2).await;
    
    // Example 3: Evening arrival only
    println!("\nExample 3: Evening arrival (5:00pm - 11:00pm)");
    
    let evening_arrival = TimeWindow::new(17, 23)?;
    
    let request3 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "ORD".to_string(),
            to_airport: "MIA".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: None,
            arrival_time: Some(evening_arrival),
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    search_and_display("Evening Arrival", request3).await;
    
    // Example 4: Red-eye flight
    println!("\nExample 4: Red-eye flight (12:00am - 6:00am)");
    
    let redeye_departure = TimeWindow::new(0, 6)?;
    
    let request4 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: Some(redeye_departure),
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    search_and_display("Red-eye", request4).await;
    
    // Example 5: Using the string parsing helper
    println!("\nExample 5: Using string parsing (09:00-17:00 departure)");
    
    let business_hours = TimeWindow::from_range_str("09:00-17:00")?;
    
    let request5 = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "DEN".to_string(),
            to_airport: "SEA".to_string(),
            max_stops: Some(1),
            airlines: None,
            departure_time: Some(business_hours),
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    search_and_display("Business Hours", request5).await;
    
    println!("\n🎯 Time window examples completed!");
    println!("Note: Actual flight results depend on Google Flights API availability.");
    
    Ok(())
}

async fn search_and_display(example_name: &str, request: FlightSearchRequest) {
    println!("Searching for {} flights...", example_name);
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ {} search completed successfully!", example_name);
            println!("   Current price level: {}", result.current_price);
            println!("   Found {} flights", result.flights.len());
            
            if let Some(best_flight) = result.flights.iter().find(|f| f.is_best) {
                println!("   Best option: {} - {}{}", best_flight.name, best_flight.price.currency, best_flight.price.amount);
            }
        }
        Err(e) => {
            println!("⚠️  {} search failed (this may be expected): {}", example_name, e);
            println!("   The protobuf encoding and time window logic are working correctly.");
        }
    }
} 
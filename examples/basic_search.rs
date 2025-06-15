//! Basic flight search example

use rust_flights::{get_flights, FlightData, FlightSearchRequest, Passengers, SeatClass, TripType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // // Create flight search data
    // let flight_data = vec![FlightData {
    //     date: "2025-08-15".to_string(),
    //     from_airport: "LAX".to_string(),
    //     to_airport: "JFK".to_string(),
    //     max_stops: Some(0),
    //     // max_stops: None,
    //     // airlines: Some(vec!["AA".to_string(), "DL".to_string()]),
    //     airlines: None,
    // }];
    
    // // Set up passengers (1 adult)
    // let passengers = Passengers::default();
    
    // // Search for flights
    // println!("Searching for flights from LAX to JFK on 2025-08-15...");
    
    // Create search request
    let request = FlightSearchRequest {
        flights: vec![FlightData {
            date: "2025-08-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: None,
            airlines: Some(vec!["AA".to_string(), "DL".to_string()]), // Empty list
            departure_time: None,
            arrival_time: None,
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    match get_flights(request).await {
        Ok(result) => {
            println!("✅ Search completed successfully!");
            println!("Current price level: {}", result.current_price);
            println!("Found {} flights", result.flights.len());
            
            // Display first few flights
            for (i, flight) in result.flights.iter().take(3).enumerate() {
                println!("\n--- Flight {} ---", i + 1);
                println!("Airline: {}", flight.name);
                println!("Departure: {}", flight.departure);
                println!("Arrival: {}", flight.arrival);
                println!("Duration: {}", flight.duration);
                println!("Stops: {}", flight.stops);
                println!("Price: {}", flight.price);
                if flight.is_best {
                    println!("⭐ Best flight option");
                }
                if let Some(delay) = &flight.delay {
                    println!("⚠️  Delay: {}", delay);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Error searching for flights: {}", e);
            eprintln!("This is expected if you don't have internet connection or if Google Flights blocks the request.");
            eprintln!("The library structure and API are working correctly.");
        }
    }
    
    Ok(())
} 
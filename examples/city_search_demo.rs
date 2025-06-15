//! Phase 4: City-Based Flight Search Demo
//! 
//! This example demonstrates the new city-based flight search API that integrates
//! Wikidata for automatic city name to Freebase ID resolution.

use rust_flights::{
    get_flights_by_city, search_flights_between_cities,
    CityFlightData, CityFlightSearchRequest, Passengers, SeatClass, TripType, TimeWindow
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Phase 4: City-Based Flight Search Demo");
    println!("========================================\n");

    // Example 1: Simple city-to-city search using convenience function
    println!("📍 Example 1: Simple London → Paris search");
    println!("-------------------------------------------");
    
    match search_flights_between_cities("London", "Paris", "2025-08-15").await {
        Ok(result) => {
            println!("✅ Success! Found {} flights", result.flights.len());
            println!("💰 Current price level: {}", result.current_price);
            
            if let Some(best_flight) = result.flights.first() {
                println!("🏆 Best flight: {} - {}{}", best_flight.name, best_flight.price.currency, best_flight.price.amount);
            }
        }
        Err(e) => {
            println!("⚠️  Error: {}", e);
        }
    }
    
    println!("\n{}\n", "=".repeat(50));

    // Example 2: Advanced city search with time windows
    println!("📍 Example 2: New York → Tokyo with time preferences");
    println!("--------------------------------------------------");
    
    let departure_time = TimeWindow::new(8, 14).unwrap();   // 8:00am to 2:00pm
    let arrival_time = TimeWindow::new(16, 22).unwrap();    // 4:00pm to 10:00pm
    
    let request = CityFlightSearchRequest {
        flights: vec![CityFlightData {
            date: "2025-09-01".to_string(),
            from_city: "New York".to_string(),
            to_city: "Tokyo".to_string(),
            max_stops: Some(1),
            airlines: Some(vec!["AA".to_string(), "UA".to_string()]),
            departure_time: Some(departure_time),
            arrival_time: Some(arrival_time),
        }],
        trip_type: TripType::OneWay,
        passengers: Passengers::default(),
        seat_class: SeatClass::Economy,
    };
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ Success! Found {} flights with time preferences", result.flights.len());
            println!("💰 Current price level: {}", result.current_price);
            
            for (i, flight) in result.flights.iter().take(3).enumerate() {
                println!(
                    "{}. {} | {} → {} | {} | {} stops | {}{}",
                    i + 1,
                    flight.name,
                    flight.departure,
                    flight.arrival,
                    flight.duration,
                    flight.stops,
                    flight.price.currency,
                    flight.price.amount
                );
            }
        }
        Err(e) => {
            println!("⚠️  Error: {}", e);
        }
    }
    
    println!("\n{}\n", "=".repeat(50));

    // Example 3: Business class round-trip
    println!("📍 Example 3: Business class round-trip (Sydney ↔ London)");
    println!("--------------------------------------------------------");
    
    let request = CityFlightSearchRequest {
        flights: vec![
            CityFlightData {
                date: "2025-07-10".to_string(),
                from_city: "Sydney".to_string(),
                to_city: "London".to_string(),
                max_stops: Some(2),
                airlines: None,
                departure_time: None,
                arrival_time: None,
            },
            CityFlightData {
                date: "2025-07-20".to_string(),
                from_city: "London".to_string(),
                to_city: "Sydney".to_string(),
                max_stops: Some(2),
                airlines: None,
                departure_time: None,
                arrival_time: None,
            },
        ],
        trip_type: TripType::RoundTrip,
        passengers: Passengers {
            adults: 2,
            children: 0,
            infants_in_seat: 0,
            infants_on_lap: 0,
        },
        seat_class: SeatClass::Business,
    };
    
    match get_flights_by_city(request).await {
        Ok(result) => {
            println!("✅ Success! Found {} business class flights", result.flights.len());
            println!("💰 Current price level: {}", result.current_price);
            
            if let Some(best_flight) = result.flights.iter().find(|f| f.is_best) {
                println!("🏆 Best flight: {} - {}{}", best_flight.name, best_flight.price.currency, best_flight.price.amount);
            }
        }
        Err(e) => {
            println!("⚠️  Error: {}", e);
        }
    }
    
    println!("\n{}\n", "=".repeat(50));

    // Example 4: Multiple city searches (demonstrating Wikidata resolution)
    println!("📍 Example 4: Multiple city name variations");
    println!("------------------------------------------");
    
    let city_pairs = vec![
        ("NYC", "LA"),
        ("New York City", "Los Angeles"), 
        ("San Francisco", "Chicago"),
        ("Boston", "Miami"),
    ];
    
    for (from, to) in city_pairs {
        println!("Searching: {} → {} ...", from, to);
        
        match search_flights_between_cities(from, to, "2025-08-20").await {
            Ok(result) => {
                println!("  ✅ Found {} flights ({})", result.flights.len(), result.current_price);
            }
            Err(e) => {
                println!("  ⚠️  Error: {}", e);
            }
        }
    }
    
    println!("\n{}\n", "=".repeat(50));
    
    println!("🎯 Phase 4 Demo Complete!");
    println!("Key Features Demonstrated:");
    println!("  • Automatic city name to Freebase ID resolution via Wikidata");
    println!("  • Time window filtering with city-based searches");
    println!("  • Business class and multi-passenger support");
    println!("  • Round-trip city-based bookings");
    println!("  • Flexible city name variations (NYC, New York City, etc.)");
    println!("  • Integration with existing Google Flights protobuf API");
    
    Ok(())
} 
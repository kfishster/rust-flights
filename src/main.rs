//! CLI interface for rust-flights

use clap::{Parser, Subcommand};
use rust_flights::{get_flights, FlightData, FlightSearchRequest, Passengers, SeatClass, TripType, TimeWindow};
use std::fs;

#[derive(Parser)]
#[command(name = "rust-flights")]
#[command(about = "A fast Google Flights API in Rust")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for flights
    Search {
        /// Origin airport code or city
        #[arg(short, long)]
        from: String,
        /// Destination airport code or city
        #[arg(short, long)]
        to: String,
        /// Departure date (YYYY-MM-DD)
        #[arg(short, long)]
        date: String,
        /// Return date for round trips (YYYY-MM-DD)
        #[arg(short, long)]
        return_date: Option<String>,
        /// Number of adults
        #[arg(long, default_value = "1")]
        adults: i32,
        /// Number of children
        #[arg(long, default_value = "0")]
        children: i32,
        /// Number of infants in seat
        #[arg(long, default_value = "0")]
        infants_in_seat: i32,
        /// Number of infants on lap
        #[arg(long, default_value = "0")]
        infants_on_lap: i32,
        /// Seat class (economy, premium-economy, business, first)
        #[arg(long, default_value = "economy")]
        class: String,
        /// Maximum number of stops
        #[arg(long)]
        max_stops: Option<i32>,
        /// Preferred airlines (comma-separated)
        #[arg(long)]
        airlines: Option<String>,
        /// Departure time window (HH:MM-HH:MM, e.g., "06:00-12:00")
        #[arg(long)]
        departure_time: Option<String>,
        /// Arrival time window (HH:MM-HH:MM, e.g., "15:00-21:00")
        #[arg(long)]
        arrival_time: Option<String>,
        /// Output file for JSON results
        #[arg(short, long)]
        output: Option<String>,
        /// Trip type (one-way, round-trip)
        #[arg(long, default_value = "one-way")]
        trip_type: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Search {
            from,
            to,
            date,
            return_date,
            adults,
            children,
            infants_in_seat,
            infants_on_lap,
            class,
            max_stops,
            airlines,
            departure_time,
            arrival_time,
            output,
            trip_type,
        } => {
            // Parse airlines once
            let parsed_airlines = airlines.map(|a| a.split(',').map(|s| s.trim().to_string()).collect());
            
            // Parse time windows
            let departure_time_window = if let Some(time_str) = departure_time {
                Some(TimeWindow::from_range_str(&time_str)?)
            } else {
                None
            };
            
            let arrival_time_window = if let Some(time_str) = arrival_time {
                Some(TimeWindow::from_range_str(&time_str)?)
            } else {
                None
            };
            
            // Build flight data
            let mut flight_data = vec![FlightData {
                date: date.clone(),
                from_airport: from.clone(),
                to_airport: to.clone(),
                max_stops,
                airlines: parsed_airlines.clone(),
                departure_time: departure_time_window.clone(),
                arrival_time: arrival_time_window.clone(),
            }];
            
            // Determine trip type and add return flight if needed
            let trip_type = if let Some(ref return_date_str) = return_date {
                flight_data.push(FlightData {
                    date: return_date_str.clone(),
                    from_airport: to,
                    to_airport: from,
                    max_stops,
                    airlines: parsed_airlines,
                    departure_time: departure_time_window,  // Apply same time preferences to return flight
                    arrival_time: arrival_time_window,
                });
                TripType::RoundTrip
            } else {
                trip_type.parse::<TripType>()?
            };
            
            // Build passengers
            let passengers = Passengers {
                adults,
                children,
                infants_in_seat,
                infants_on_lap,
            };
            let seat_class = class.parse::<SeatClass>()?;
            
            // Create search request
            let request = FlightSearchRequest {
                flights: flight_data,
                trip_type,
                passengers,
                seat_class,
            };
            
            // Search for flights
            println!("Searching for flights...");
            match get_flights(request).await {
                Ok(result) => {
                    // Output results
                    let json = serde_json::to_string_pretty(&result)?;
                    
                    if let Some(output_file) = output {
                        fs::write(&output_file, &json)?;
                        println!("Results saved to {}", output_file);
                    } else {
                        println!("{}", json);
                    }
                    
                    // Print summary
                    println!("\nSummary:");
                    println!("Current price level: {}", result.current_price);
                    println!("Found {} flights", result.flights.len());
                    
                    if !result.flights.is_empty() {
                        let best_flight = &result.flights[0];
                        println!("Best flight: {} - {}", best_flight.name, best_flight.price);
                    }
                }
                Err(e) => {
                    eprintln!("Error searching for flights: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test basic search command
        let cli = Cli::try_parse_from(&[
            "rust-flights",
            "search",
            "--from", "LAX",
            "--to", "JFK",
            "--date", "2024-01-15",
        ]);
        
        assert!(cli.is_ok());
        
        if let Ok(Cli { command: Commands::Search { from, to, date, .. } }) = cli {
            assert_eq!(from, "LAX");
            assert_eq!(to, "JFK");
            assert_eq!(date, "2024-01-15");
        }
    }
    
    #[test]
    fn test_cli_parsing_with_time_windows() {
        // Test search command with time windows
        let cli = Cli::try_parse_from(&[
            "rust-flights",
            "search",
            "--from", "LAX",
            "--to", "JFK",
            "--date", "2024-01-15",
            "--departure-time", "06:00-12:00",
            "--arrival-time", "15:00-21:00",
        ]);
        
        assert!(cli.is_ok());
        
        if let Ok(Cli { command: Commands::Search { departure_time, arrival_time, .. } }) = cli {
            assert_eq!(departure_time, Some("06:00-12:00".to_string()));
            assert_eq!(arrival_time, Some("15:00-21:00".to_string()));
        }
    }
} 
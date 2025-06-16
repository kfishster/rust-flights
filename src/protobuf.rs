//! Protobuf definitions and utilities for Google Flights API

use crate::{FlightError, Passengers, SeatClass, TripType};
use prost::Message;
use base64::{Engine as _, engine::general_purpose};

// Include the generated protobuf code
include!(concat!(env!("OUT_DIR"), "/_.rs"));

// Alias our FlightData to avoid naming conflict with protobuf FlightData
use crate::FlightData as ApiFlightData;

impl From<SeatClass> for Seat {
    fn from(seat_class: SeatClass) -> Self {
        match seat_class {
            SeatClass::Economy => Seat::Economy,
            SeatClass::PremiumEconomy => Seat::PremiumEconomy,
            SeatClass::Business => Seat::Business,
            SeatClass::First => Seat::First,
        }
    }
}

impl From<TripType> for Trip {
    fn from(trip_type: TripType) -> Self {
        match trip_type {
            TripType::RoundTrip => Trip::RoundTrip,
            TripType::OneWay => Trip::OneWay,
            TripType::MultiCity => Trip::MultiCity,
        }
    }
}

impl From<Passengers> for Vec<Passenger> {
    fn from(passengers: Passengers) -> Self {
        let mut result = Vec::new();
        
        // Add adults
        for _ in 0..passengers.adults {
            result.push(Passenger::Adult);
        }
        
        // Add children
        for _ in 0..passengers.children {
            result.push(Passenger::Child);
        }
        
        // Add infants in seat
        for _ in 0..passengers.infants_in_seat {
            result.push(Passenger::InfantInSeat);
        }
        
        // Add infants on lap
        for _ in 0..passengers.infants_on_lap {
            result.push(Passenger::InfantOnLap);
        }
        
        result
    }
}

/// Build protobuf Info message from flight search parameters
pub fn build_flight_info(
    flight_data: Vec<ApiFlightData>,
    trip_type: TripType,
    passengers: Passengers,
    seat_class: SeatClass,
) -> Result<Info, FlightError> {
    let mut proto_flight_data = Vec::new();
    
    for flight in flight_data {
        let from_airport = Airport {
            airport: flight.from_airport,
        };
        
        let to_airport = Airport {
            airport: flight.to_airport,
        };
        
        let proto_flight = FlightData {
            date: flight.date,
            from_flight: Some(from_airport),
            to_flight: Some(to_airport),
            max_stops: flight.max_stops,
            airlines: flight.airlines.unwrap_or_default(),
            departure_earliest_hour: flight.departure_time.as_ref().map(|t| t.earliest_hour),
            departure_latest_hour: flight.departure_time.as_ref().map(|t| t.latest_hour),
            arrival_earliest_hour: flight.arrival_time.as_ref().map(|t| t.earliest_hour),
            arrival_latest_hour: flight.arrival_time.as_ref().map(|t| t.latest_hour),
            selected_flight: None,
        };
        
        proto_flight_data.push(proto_flight);
    }
    
    let passenger_list: Vec<Passenger> = passengers.into();
    let passenger_ints: Vec<i32> = passenger_list.iter().map(|p| *p as i32).collect();
    
    Ok(Info {
        data: proto_flight_data,
        seat: Into::<Seat>::into(seat_class) as i32,
        passengers: passenger_ints,
        trip: Into::<Trip>::into(trip_type) as i32,
    })
}

/// Selected flight information for building itinerary links
#[derive(Debug, Clone)]
pub struct SelectedFlight {
    pub from_airport: String,
    pub to_airport: String,
    pub departure_date: String,
    pub airline_code: String,
    pub flight_number: String,
}

/// Build protobuf Info message with selected flights for itinerary links
pub fn build_itinerary_info(
    selected_flights: Vec<SelectedFlight>,
    trip_type: TripType,
    passengers: Passengers,
    seat_class: SeatClass,
) -> Result<Info, FlightError> {
    let mut proto_flight_data = Vec::new();
    
    for flight in selected_flights {
        let from_airport = Airport {
            airport: flight.from_airport.clone(),
        };
        
        let to_airport = Airport {
            airport: flight.to_airport.clone(),
        };
        
        let selected_flight_data = SelectedFlightData {
            from_airport: flight.from_airport.clone(),
            departure_date: flight.departure_date.clone(),
            to_airport: flight.to_airport.clone(),
            airline_code: flight.airline_code,
            flight_number: flight.flight_number,
        };
        
        let proto_flight = FlightData {
            date: flight.departure_date,
            from_flight: Some(from_airport),
            to_flight: Some(to_airport),
            max_stops: None,
            airlines: vec![],
            selected_flight: Some(selected_flight_data),
            departure_earliest_hour: None,
            departure_latest_hour: None,
            arrival_earliest_hour: None,
            arrival_latest_hour: None,
        };
        
        proto_flight_data.push(proto_flight);
    }
    
    let passenger_list: Vec<Passenger> = passengers.into();
    let passenger_ints: Vec<i32> = passenger_list.iter().map(|p| *p as i32).collect();
    
    Ok(Info {
        data: proto_flight_data,
        seat: Into::<Seat>::into(seat_class) as i32,
        passengers: passenger_ints,
        trip: Into::<Trip>::into(trip_type) as i32,
    })
}

/// Encode protobuf message to base64 for URL parameter
pub fn encode_to_base64(info: &Info) -> Result<String, FlightError> {
    let mut buf = Vec::new();
    info.encode(&mut buf)?;
    Ok(general_purpose::URL_SAFE.encode(&buf))
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_seat_class_conversion() {
        assert_eq!(Seat::from(SeatClass::Economy) as i32, Seat::Economy as i32);
        assert_eq!(Seat::from(SeatClass::PremiumEconomy) as i32, Seat::PremiumEconomy as i32);
        assert_eq!(Seat::from(SeatClass::Business) as i32, Seat::Business as i32);
        assert_eq!(Seat::from(SeatClass::First) as i32, Seat::First as i32);
    }

    #[test]
    fn test_trip_type_conversion() {
        assert_eq!(Trip::from(TripType::RoundTrip) as i32, Trip::RoundTrip as i32);
        assert_eq!(Trip::from(TripType::OneWay) as i32, Trip::OneWay as i32);
        assert_eq!(Trip::from(TripType::MultiCity) as i32, Trip::MultiCity as i32);
    }

    #[test]
    fn test_passengers_conversion() {
        let passengers = Passengers {
            adults: 2,
            children: 1,
            infants_in_seat: 0,
            infants_on_lap: 1,
        };
        
        let passenger_vec: Vec<Passenger> = passengers.into();
        assert_eq!(passenger_vec.len(), 4);
        assert_eq!(passenger_vec[0], Passenger::Adult);
        assert_eq!(passenger_vec[1], Passenger::Adult);
        assert_eq!(passenger_vec[2], Passenger::Child);
        assert_eq!(passenger_vec[3], Passenger::InfantOnLap);
    }

    #[test]
    fn test_build_flight_info() {
        let flight_data = vec![ApiFlightData {
            date: "2024-01-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: Some(vec!["AA".to_string()]),
            departure_time: None,
            arrival_time: None,
        }];
        
        let passengers = Passengers::default();
        let info = build_flight_info(
            flight_data,
            TripType::OneWay,
            passengers,
            SeatClass::Economy,
        ).unwrap();
        
        assert_eq!(info.data.len(), 1);
        assert_eq!(info.data[0].date, "2024-01-15");
        assert_eq!(info.data[0].from_flight.as_ref().unwrap().airport, "LAX");
        assert_eq!(info.data[0].to_flight.as_ref().unwrap().airport, "JFK");
        assert_eq!(info.data[0].max_stops, Some(1));
        assert_eq!(info.data[0].airlines, vec!["AA"]);
        assert_eq!(info.data[0].departure_earliest_hour, None);
        assert_eq!(info.data[0].departure_latest_hour, None);
        assert_eq!(info.data[0].arrival_earliest_hour, None);
        assert_eq!(info.data[0].arrival_latest_hour, None);
    }
    
    #[test]
    fn test_build_flight_info_with_time_windows() {
        use crate::TimeWindow;
        
        let departure_time = TimeWindow::new(8, 12).unwrap();
        let arrival_time = TimeWindow::new(15, 20).unwrap();
        
        let flight_data = vec![ApiFlightData {
            date: "2024-01-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: Some(1),
            airlines: Some(vec!["AA".to_string()]),
            departure_time: Some(departure_time),
            arrival_time: Some(arrival_time),
        }];
        
        let passengers = Passengers::default();
        let info = build_flight_info(
            flight_data,
            TripType::OneWay,
            passengers,
            SeatClass::Economy,
        ).unwrap();
        
        assert_eq!(info.data.len(), 1);
        assert_eq!(info.data[0].departure_earliest_hour, Some(8));
        assert_eq!(info.data[0].departure_latest_hour, Some(12));
        assert_eq!(info.data[0].arrival_earliest_hour, Some(15));
        assert_eq!(info.data[0].arrival_latest_hour, Some(20));
    }
    
    #[test]
    fn test_time_window_encoding_example() {
        use crate::TimeWindow;
        
        // Example from user: flight should leave between 12:00am and 11:00am, and arrive 3pm-12am
        let departure_time = TimeWindow::new(0, 11).unwrap(); // 12:00am to 11:00am 
        let arrival_time = TimeWindow::new(15, 23).unwrap();  // 3:00pm to 11:00pm (23 = 11pm)
        
        let flight_data = vec![ApiFlightData {
            date: "2024-01-15".to_string(),
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            max_stops: None,
            airlines: None,
            departure_time: Some(departure_time),
            arrival_time: Some(arrival_time),
        }];
        
        let passengers = Passengers::default();
        let info = build_flight_info(
            flight_data,
            TripType::OneWay,
            passengers,
            SeatClass::Economy,
        ).unwrap();
        
        // Verify the exact values mentioned in the user's example
        assert_eq!(info.data[0].departure_earliest_hour, Some(0));   // Field 8: 0 (12:00am)
        assert_eq!(info.data[0].departure_latest_hour, Some(11));    // Field 9: 11 (11:00am) 
        assert_eq!(info.data[0].arrival_earliest_hour, Some(15));    // Field 10: 15 (3:00pm)
        assert_eq!(info.data[0].arrival_latest_hour, Some(23));      // Field 11: 23 (11:00pm)
    }

    #[test]
    fn test_encode_to_base64() {
        let info = Info {
            data: vec![],
            seat: Seat::Economy as i32,
            passengers: vec![Passenger::Adult as i32],
            trip: Trip::OneWay as i32,
        };
        
        let encoded = encode_to_base64(&info).unwrap();
        assert!(!encoded.is_empty());
        // Base64 should be valid
        assert!(general_purpose::STANDARD.decode(&encoded).is_ok());
    }

    #[test]
    fn test_passenger_enum_values() {
        // Test that our enum values match expected protobuf values
        assert_eq!(Passenger::Adult as i32, 1);
        assert_eq!(Passenger::Child as i32, 2);
        assert_eq!(Passenger::InfantInSeat as i32, 3);
        assert_eq!(Passenger::InfantOnLap as i32, 4);
        
        // Test building passenger vector
        let passengers = Passengers {
            adults: 1,
            children: 0,
            infants_in_seat: 0,
            infants_on_lap: 0,
        };
        
        let passenger_list: Vec<Passenger> = passengers.into();
        let passenger_ints: Vec<i32> = passenger_list.iter().map(|p| *p as i32).collect();
        
        assert_eq!(passenger_ints, vec![1]); // Should be [1] for one adult
        
        // Test encoding a minimal Info with passengers
        let info = Info {
            data: vec![],
            seat: Seat::Economy as i32,
            passengers: passenger_ints,
            trip: Trip::OneWay as i32,
        };
        
        let encoded = encode_to_base64(&info).unwrap();
        println!("Encoded with 1 adult: {}", encoded);
        
        // Decode to verify
        let decoded_bytes = general_purpose::STANDARD.decode(&encoded).unwrap();
        let decoded_info = Info::decode(&decoded_bytes[..]).unwrap();
        assert_eq!(decoded_info.passengers, vec![1]);
    }

    #[test]
    fn test_detailed_protobuf_structure() {
        // Create Info with all fields populated for testing
        let info = Info {
            data: vec![], // field 3
            seat: Seat::Economy as i32, // field 9 = 1
            passengers: vec![1], // field 8 = [1] (one adult)
            trip: Trip::OneWay as i32, // field 19 = 2
        };
        
        println!("Info struct:");
        println!("  data (field 3): {:?}", info.data);
        println!("  seat (field 9): {} (Economy)", info.seat);
        println!("  passengers (field 8): {:?} (Adult=1)", info.passengers);
        println!("  trip (field 19): {} (OneWay)", info.trip);
        
        let encoded = encode_to_base64(&info).unwrap();
        println!("Base64: {}", encoded);
        
        // Decode and verify each field
        let decoded_bytes = general_purpose::STANDARD.decode(&encoded).unwrap();
        let decoded_info = Info::decode(&decoded_bytes[..]).unwrap();
        
        println!("Decoded back:");
        println!("  passengers field 8: {:?}", decoded_info.passengers);
        println!("  seat field 9: {}", decoded_info.seat);
        println!("  trip field 19: {}", decoded_info.trip);
        
        assert_eq!(decoded_info.passengers, vec![1]);
        assert_eq!(decoded_info.seat, 1);
        assert_eq!(decoded_info.trip, 2);
    }

    #[test]
    fn test_build_itinerary_info_with_passengers() {
        use crate::{TripType, SeatClass, Passengers};
        
        let selected_flights = vec![SelectedFlight {
            from_airport: "LAX".to_string(),
            to_airport: "JFK".to_string(),
            departure_date: "2024-01-15".to_string(),
            airline_code: "AA".to_string(),
            flight_number: "123".to_string(),
        }];
        
        let passengers = Passengers {
            adults: 1,
            children: 0,
            infants_in_seat: 0,
            infants_on_lap: 0,
        };
        
        let info = build_itinerary_info(
            selected_flights,
            TripType::OneWay,
            passengers,
            SeatClass::Economy,
        ).unwrap();
        
        println!("build_itinerary_info result:");
        println!("  passengers field 8: {:?}", info.passengers);
        println!("  seat field 9: {}", info.seat);
        println!("  trip field 19: {}", info.trip);
        println!("  data length: {}", info.data.len());
        if let Some(flight) = info.data.first() {
            println!("  first flight has selected_flight: {}", flight.selected_flight.is_some());
        }
        
        let encoded = encode_to_base64(&info).unwrap();
        println!("Itinerary Base64: {}", encoded);
        
        // Verify passengers field specifically
        assert_eq!(info.passengers, vec![1]); // Should be [1] for one adult
        assert_eq!(info.seat, 1); // Economy
        assert_eq!(info.trip, 2); // OneWay
    }
} 
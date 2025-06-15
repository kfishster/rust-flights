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
    Ok(general_purpose::STANDARD.encode(&buf))
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
} 
//! HTTP client and HTML parser for Google Flights

use crate::{Flight, FlightError, FlightResult, FlightSearchRequest};
use crate::protobuf::{build_flight_info, encode_to_base64};
use reqwest::Client;
use scraper::{Html, Selector};

/// Main flight client for making requests to Google Flights
pub struct FlightClient {
    http_client: Client,
}

impl FlightClient {
    /// Create a new flight client
    pub async fn new() -> Result<Self, FlightError> {
        let http_client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;
        
        Ok(Self { http_client })
    }

    /// Main API function with consolidated parameters
    pub async fn get_flights(&self, request: FlightSearchRequest) -> Result<FlightResult, FlightError> {
        // Build protobuf message
        let info = build_flight_info(request.flights, request.trip_type, request.passengers, request.seat_class)?;
        
        // Encode to base64
        let encoded = encode_to_base64(&info)?;
        
        println!("Encoded: {}", encoded);

        // Build URL
        let url = format!("https://www.google.com/travel/flights?tfs={}", encoded);
        
        // Make HTTP request
        let response = self.http_client.get(&url).send().await?;
        let html = response.text().await?;
        
        // Parse response
        let parser = FlightResponseParser::new()?;
        parser.parse_response(&html)
    }
}

/// HTML parser for Google Flights responses
pub struct FlightResponseParser {
    // Pre-compiled selectors from Python implementation
    flights_selector: Selector,           // div[jsname="IWWDBc"], div[jsname="YdtKid"]
    flight_items_selector: Selector,      // ul.Rk10dc li
    flight_name_selector: Selector,       // div.sSHqwe.tPgKwe.ogfYpf span
    departure_arrival_selector: Selector, // span.mv1WYe div
    time_ahead_selector: Selector,        // span.bOzv6
    duration_selector: Selector,          // li div.Ak5kof div
    stops_selector: Selector,             // .BbR8Ec .ogfYpf
    delay_selector: Selector,             // .GsCCve
    price_selector: Selector,             // .YMlIz.FpEdX
    current_price_selector: Selector,     // span.gOatQ
}

impl FlightResponseParser {
    pub fn new() -> Result<Self, FlightError> {
        Ok(Self {
            flights_selector: Selector::parse(r#"div[jsname="IWWDBc"], div[jsname="YdtKid"]"#)
                .map_err(|e| FlightError::ParseError(format!("Invalid flights selector: {}", e)))?,
            flight_items_selector: Selector::parse("ul.Rk10dc li")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight items selector: {}", e)))?,
            flight_name_selector: Selector::parse("div.sSHqwe.tPgKwe.ogfYpf span")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight name selector: {}", e)))?,
            departure_arrival_selector: Selector::parse("span.mv1WYe div")
                .map_err(|e| FlightError::ParseError(format!("Invalid departure/arrival selector: {}", e)))?,
            time_ahead_selector: Selector::parse("span.bOzv6")
                .map_err(|e| FlightError::ParseError(format!("Invalid time ahead selector: {}", e)))?,
            duration_selector: Selector::parse("li div.Ak5kof div")
                .map_err(|e| FlightError::ParseError(format!("Invalid duration selector: {}", e)))?,
            stops_selector: Selector::parse(".BbR8Ec .ogfYpf")
                .map_err(|e| FlightError::ParseError(format!("Invalid stops selector: {}", e)))?,
            delay_selector: Selector::parse(".GsCCve")
                .map_err(|e| FlightError::ParseError(format!("Invalid delay selector: {}", e)))?,
            price_selector: Selector::parse(".YMlIz.FpEdX")
                .map_err(|e| FlightError::ParseError(format!("Invalid price selector: {}", e)))?,
            current_price_selector: Selector::parse("span.gOatQ")
                .map_err(|e| FlightError::ParseError(format!("Invalid current price selector: {}", e)))?,
        })
    }

    pub fn parse_response(&self, html: &str) -> Result<FlightResult, FlightError> {
        let document = Html::parse_document(html);
        let flights = self.extract_flights(&document)?;
        let current_price = self.extract_current_price(&document);
        
        Ok(FlightResult {
            current_price,
            flights,
        })
    }
    
    fn extract_flights(&self, document: &Html) -> Result<Vec<Flight>, FlightError> {
        let mut flights = Vec::new();
        
        for (i, flight_section) in document.select(&self.flights_selector).enumerate() {
            let is_best_flight = i == 0;
            
            // Get flight items, excluding last item for non-best flights (Python logic)
            let flight_items: Vec<_> = flight_section.select(&self.flight_items_selector).collect();
            let items_to_process = if is_best_flight { 
                flight_items 
            } else { 
                let len = flight_items.len();
                flight_items.into_iter().take(len.saturating_sub(1)).collect()
            };
            
            for item in items_to_process {
                // Extract flight name (critical)
                let name = item.select(&self.flight_name_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Flight name not found for item");
                        "Unknown".to_string()
                    });
                
                // Extract departure & arrival times (critical)
                let times: Vec<String> = item.select(&self.departure_arrival_selector)
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .collect();
                let departure = times.first().cloned().unwrap_or_else(|| {
                    eprintln!("⚠️  Warning: Departure time not found for flight: {}", name);
                    "Unknown".to_string()
                });
                let arrival = times.get(1).cloned().unwrap_or_else(|| {
                    eprintln!("⚠️  Warning: Arrival time not found for flight: {}", name);
                    "Unknown".to_string()
                });
                
                // Extract time ahead (non-critical)
                let arrival_time_ahead = item.select(&self.time_ahead_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default();
                
                // Extract duration (critical)
                let duration = item.select(&self.duration_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Duration not found for flight: {}", name);
                        "Unknown".to_string()
                    });
                
                // Extract stops (critical)
                let stops_text = item.select(&self.stops_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Stops information not found for flight: {}", name);
                        "Unknown".to_string()
                    });
                
                let stops = if stops_text == "Nonstop" {
                    0
                } else if stops_text == "Unknown" {
                    -1 // Unknown format
                } else {
                    stops_text.split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(-1) // Unknown format
                };
                
                // Extract delay (non-critical)
                let delay = item.select(&self.delay_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .filter(|s| !s.is_empty());
                
                // Extract price (critical)
                let price = item.select(&self.price_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().replace(',', ""))
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Price not found for flight: {}", name);
                        "0".to_string()
                    });
                
                flights.push(Flight {
                    is_best: is_best_flight,
                    name,
                    departure: departure.split_whitespace().collect::<Vec<_>>().join(" "),
                    arrival: arrival.split_whitespace().collect::<Vec<_>>().join(" "),
                    arrival_time_ahead,
                    duration,
                    stops,
                    delay,
                    price,
                });
            }
        }
        
        if flights.is_empty() {
            return Err(FlightError::ParseError("No flights found in response".to_string()));
        }
        
        Ok(flights)
    }
    
    fn extract_current_price(&self, document: &Html) -> String {
        match document.select(&self.current_price_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .filter(|s| !s.is_empty())
        {
            Some(price) => price,
            None => {
                eprintln!("⚠️  Warning: Current price information not found (this is non-critical)");
                "unknown".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flight_client_creation() {
        let client = FlightClient::new().await;
        assert!(client.is_ok());
    }

    #[test]
    fn test_flight_response_parser_creation() {
        let parser = FlightResponseParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parse_empty_response() {
        let parser = FlightResponseParser::new().unwrap();
        let result = parser.parse_response("<html></html>");
        assert!(result.is_err());
    }
} 
//! HTTP client and HTML parser for Google Flights

use crate::{Flight, FlightError, FlightResult, FlightSearchRequest, FlightPrice};
use crate::protobuf::{build_flight_info, encode_to_base64};
use reqwest::Client;
use scraper::{Html, Selector};
use regex::Regex;
use tracing::{info, warn, error, debug, instrument};

/// Main flight client for making requests to Google Flights
pub struct FlightClient {
    http_client: Client,
}

impl FlightClient {
    /// Create a new flight client
    pub async fn new() -> Result<Self, FlightError> {
        debug!("Creating new flight client");
        let http_client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;
        
        debug!("Flight client created successfully");
        Ok(Self { http_client })
    }

    /// Main API function with consolidated parameters
    #[instrument(level = "info", skip(self, request))]
    pub async fn get_flights(&self, request: FlightSearchRequest) -> Result<FlightResult, FlightError> {
        info!("Building Google Flights request");
        
        // Build protobuf message
        let info = build_flight_info(request.flights, request.trip_type, request.passengers, request.seat_class)?;
        debug!("Protobuf message built successfully");
        
        // Encode to base64
        let encoded = encode_to_base64(&info)?;
        debug!(encoded_length = encoded.len(), "Encoded protobuf to base64");

        // Build URL
        let url = format!("https://www.google.com/travel/flights?tfs={}", encoded);
        info!(url = %url, "Making HTTP request to Google Flights");
        
        // Make HTTP request
        let start_time = std::time::Instant::now();
        let response = self.http_client.get(&url).send().await?;
        let status = response.status();
        let request_duration = start_time.elapsed();
        
        info!(
            status = %status,
            duration_ms = request_duration.as_millis(),
            "HTTP request completed"
        );
        
        if !status.is_success() {
            error!(status = %status, "HTTP request failed");
            return Err(FlightError::HttpError(reqwest::Error::from(response.error_for_status().unwrap_err())));
        }
        
        let html = response.text().await?;
        info!(html_length = html.len(), "Received HTML response");
        
        // Parse response
        let parser = FlightResponseParser::new()?;
        let start_parse = std::time::Instant::now();
        let result = parser.parse_response(&html);
        let parse_duration = start_parse.elapsed();
        
        match &result {
            Ok(flight_result) => {
                info!(
                    parse_duration_ms = parse_duration.as_millis(),
                    flights_found = flight_result.flights.len(),
                    "HTML parsing completed successfully"
                );
            },
            Err(e) => {
                error!(
                    parse_duration_ms = parse_duration.as_millis(),
                    error = %e,
                    "HTML parsing failed"
                );
            }
        }
        
        result
    }
}

/// HTML parser for Google Flights responses
pub struct FlightResponseParser {
    // Pre-compiled selectors from Python implementation
    flights_selector: Selector,           // div[jsname="IWWDBc"], div[jsname="YdtKid"]
    flight_items_selector: Selector,      // ul.Rk10dc li
    flight_name_selector: Selector,       // div.sSHqwe.tPgKwe.ogfYpf span
    departure_arrival_selector: Selector, // span.mv1WYe div
    duration_selector: Selector,          // li div.Ak5kof div
    route_selector: Selector,             // li div.Ak5kof span
    stops_selector: Selector,             // .BbR8Ec .ogfYpf
    price_selector: Selector,             // .YMlIz.FpEdX
    current_price_selector: Selector,     // span.gOatQ
    flight_info_selector: Selector,       // .NZRfve (for flight number extraction)
}

impl FlightResponseParser {
    pub fn new() -> Result<Self, FlightError> {
        debug!("Initializing HTML parser with selectors");
        Ok(Self {
            flights_selector: Selector::parse(r#"div[jsname="IWWDBc"], div[jsname="YdtKid"]"#)
                .map_err(|e| FlightError::ParseError(format!("Invalid flights selector: {}", e)))?,
            flight_items_selector: Selector::parse("ul.Rk10dc li")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight items selector: {}", e)))?,
            flight_name_selector: Selector::parse("div.sSHqwe.tPgKwe.ogfYpf span")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight name selector: {}", e)))?,
            departure_arrival_selector: Selector::parse("span.mv1WYe div")
                .map_err(|e| FlightError::ParseError(format!("Invalid departure/arrival selector: {}", e)))?,
            duration_selector: Selector::parse("li div.Ak5kof div")
                .map_err(|e| FlightError::ParseError(format!("Invalid duration selector: {}", e)))?,
            route_selector: Selector::parse("li div.Ak5kof span")
                .map_err(|e| FlightError::ParseError(format!("Invalid route selector: {}", e)))?,
            stops_selector: Selector::parse(".BbR8Ec .ogfYpf")
                .map_err(|e| FlightError::ParseError(format!("Invalid stops selector: {}", e)))?,
            price_selector: Selector::parse(".YMlIz.FpEdX")
                .map_err(|e| FlightError::ParseError(format!("Invalid price selector: {}", e)))?,
            current_price_selector: Selector::parse("span.gOatQ")
                .map_err(|e| FlightError::ParseError(format!("Invalid current price selector: {}", e)))?,
            flight_info_selector: Selector::parse(".NZRfve")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight info selector: {}", e)))?,
        })
    }

    pub fn parse_response(&self, html: &str) -> Result<FlightResult, FlightError> {
        debug!("Starting HTML parsing");
        let document = Html::parse_document(html);
        
        let flights = self.extract_flights(&document)?;
        let current_price = self.extract_current_price(&document);
        
        debug!(
            flights_extracted = flights.len(),
            current_price = current_price,
            "HTML parsing completed"
        );
        
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
                
                // Extract duration (critical)
                let duration = item.select(&self.duration_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Duration not found for flight: {}", name);
                        "Unknown".to_string()
                    });
                
                // Extract route (critical)
                let route = item.select(&self.route_selector)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Route not found for flight: {}", name);
                        "Unknown".to_string()
                    });

                // route is hyphenated, split into origin and destination
                let route_parts = route.split('-').collect::<Vec<&str>>();
                let origin = route_parts[0].to_string();
                let destination = route_parts[1].to_string();

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
                
                // Extract price (critical) and parse currency/amount
                let price_text = item.select(&self.price_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().replace(',', ""))
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Price not found for flight: {}", name);
                        "$0".to_string()
                    });
                
                let price = self.parse_price(&price_text);
                
                // Extract flight number from NZRfve class
                let (airline_code, flight_number) = self.extract_flight_info(&item);
                
                flights.push(Flight {
                    is_best: is_best_flight,
                    name,
                    departure: departure.split_whitespace().collect::<Vec<_>>().join(" "),
                    arrival: arrival.split_whitespace().collect::<Vec<_>>().join(" "),
                    duration,
                    origin,
                    destination,
                    stops,
                    price,
                    airline_code,
                    flight_number,
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

    fn parse_price(&self, price_text: &str) -> FlightPrice {
        // Extract currency symbol and amount
        let re = Regex::new(r"([^\d]+)(\d+)").unwrap();
        
        if let Some(captures) = re.captures(price_text) {
            let currency_symbol = captures.get(1).map_or("$", |m| m.as_str());
            let amount_str = captures.get(2).map_or("0", |m| m.as_str());
            let amount = amount_str.parse::<i32>().unwrap_or(0);
            
            FlightPrice { 
                amount, 
                currency: currency_symbol.trim().to_string()
            }
        } else {
            // Fallback parsing
            let amount = price_text.chars()
                .filter(|c| c.is_numeric())
                .collect::<String>()
                .parse::<i32>()
                .unwrap_or(0);
            
            FlightPrice {
                amount,
                currency: "$".to_string(),
            }
        }
    }
    
    fn extract_flight_info(&self, item: &scraper::ElementRef) -> (Option<String>, Option<String>) {
        // Look for flight info in NZRfve class with data-travelimpactmodelwebsiteurl
        if let Some(element) = item.select(&self.flight_info_selector).next() {
            if let Some(url) = element.value().attr("data-travelimpactmodelwebsiteurl") {
                return self.parse_flight_info_from_url(url);
            }
        }
        
        (None, None)
    }
    
    fn parse_flight_info_from_url(&self, url: &str) -> (Option<String>, Option<String>) {
        // Parse URLs like:
        // https://www.travelimpactmodel.org/lookup/flight?itinerary=LAX-JFK-AA-274-20250815
        // https://www.travelimpactmodel.org/lookup/flight?itinerary=LAX-ATL-F9-4316-20250815,ATL-JFK-F9-4818-20250815
        
        let re = Regex::new(r"itinerary=([^&]+)").unwrap();
        
        if let Some(captures) = re.captures(url) {
            let itinerary = captures.get(1).map_or("", |m| m.as_str());
            
            // Split by comma for multi-leg flights and take the first one
            let first_leg = itinerary.split(',').next().unwrap_or("");
            
            // Parse format: ORIGIN-DEST-AIRLINE-NUMBER-DATE
            let parts: Vec<&str> = first_leg.split('-').collect();
            
            if parts.len() >= 4 {
                let airline_code = parts[2].to_string();
                let flight_number = parts[3].to_string();
                return (Some(airline_code), Some(flight_number));
            }
        }
        
        (None, None)
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
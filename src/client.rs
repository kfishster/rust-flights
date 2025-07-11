//! HTTP client and HTML parser for Google Flights

use crate::{Flight, FlightError, FlightResult, FlightSearchRequest, FlightPrice};
use crate::protobuf::{build_flight_info, encode_to_base64};
use reqwest::Client;
use scraper::{Html, Selector};
use regex::Regex;

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
    duration_selector: Selector,          // li div.Ak5kof div
    stops_selector: Selector,             // .BbR8Ec .ogfYpf
    price_selector: Selector,             // .YMlIz.FpEdX
    current_price_selector: Selector,     // span.gOatQ
    flight_info_selector: Selector,       // .NZRfve (for flight number extraction)
    airport_codes_selector: Selector,     // span.PTuQse span[jscontroller="cNtv4b"] (for origin/destination airports)
    flight_summary_selector: Selector,     // JMc5Xc
    layover_selector: Selector,           // div.sSHqwe.tPgKwe.ogfYpf (for layover description and airports)
    layover_airport_selector: Selector,   // span[jscontroller="cNtv4b"] (for layover airport codes)
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
            duration_selector: Selector::parse("li div.Ak5kof div")
                .map_err(|e| FlightError::ParseError(format!("Invalid duration selector: {}", e)))?,
            stops_selector: Selector::parse(".BbR8Ec .ogfYpf")
                .map_err(|e| FlightError::ParseError(format!("Invalid stops selector: {}", e)))?,
            price_selector: Selector::parse(".YMlIz.FpEdX")
                .map_err(|e| FlightError::ParseError(format!("Invalid price selector: {}", e)))?,
            current_price_selector: Selector::parse("span.gOatQ")
                .map_err(|e| FlightError::ParseError(format!("Invalid current price selector: {}", e)))?,
            flight_info_selector: Selector::parse(".NZRfve")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight info selector: {}", e)))?,
            airport_codes_selector: Selector::parse("span.PTuQse span[jscontroller=\"cNtv4b\"]")
                .map_err(|e| FlightError::ParseError(format!("Invalid airport codes selector: {}", e)))?,
            flight_summary_selector: Selector::parse(".JMc5Xc")
                .map_err(|e| FlightError::ParseError(format!("Invalid flight summary selector: {}", e)))?,
            layover_selector: Selector::parse("div.sSHqwe.tPgKwe.ogfYpf")
                .map_err(|e| FlightError::ParseError(format!("Invalid layover selector: {}", e)))?,
            layover_airport_selector: Selector::parse("span[jscontroller=\"cNtv4b\"]")
                .map_err(|e| FlightError::ParseError(format!("Invalid layover airport selector: {}", e)))?,
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
                
                // Extract price (critical) and parse currency/amount
                let price_text = item.select(&self.price_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().replace(',', ""))
                    .unwrap_or_else(|| {
                        eprintln!("⚠️  Warning: Price not found for flight: {}", name);
                        "$0".to_string()
                    });
                
                let price = self.parse_price(&price_text);
                
                // Extract flight legs from NZRfve class
                let flight_legs = self.extract_flight_info(&item);
                
                // Extract origin and destination airport codes
                let (origin_airport, destination_airport) = self.extract_airport_codes(&item);

                // Extract flight summary
                let flight_summary = item.select(&self.flight_summary_selector)
                    .next()
                    .map(|el| el.value().attr("aria-label").unwrap_or("").to_string());

                // Extract layover information
                let (layovers, layover_description) = self.extract_layover_info(&item);

                flights.push(Flight {
                    is_best: is_best_flight,
                    name,
                    departure: departure.split_whitespace().collect::<Vec<_>>().join(" "),
                    arrival: arrival.split_whitespace().collect::<Vec<_>>().join(" "),
                    duration,
                    stops,
                    price,
                    flight_legs,
                    origin_airport,
                    destination_airport,
                    flight_summary,
                    layovers,
                    layover_description,
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
    
    fn extract_flight_info(&self, item: &scraper::ElementRef) -> Option<Vec<crate::FlightLeg>> {
        // Look for flight info in NZRfve class with data-travelimpactmodelwebsiteurl
        if let Some(element) = item.select(&self.flight_info_selector).next() {
            if let Some(url) = element.value().attr("data-travelimpactmodelwebsiteurl") {
                return self.parse_flight_info_from_url(url);
            }
        }
        
        None
    }
    
    fn parse_flight_info_from_url(&self, url: &str) -> Option<Vec<crate::FlightLeg>> {
        // Parse URLs like:
        // https://www.travelimpactmodel.org/lookup/flight?itinerary=LAX-JFK-AA-274-20250815
        // https://www.travelimpactmodel.org/lookup/flight?itinerary=LAX-ATL-F9-4316-20250815,ATL-JFK-F9-4818-20250815
        
        let re = Regex::new(r"itinerary=([^&]+)").unwrap();
        
        if let Some(captures) = re.captures(url) {
            let itinerary = captures.get(1).map_or("", |m| m.as_str());
            
            let mut flight_legs = Vec::new();
            
            // Split by comma for multi-leg flights and process all legs
            for leg in itinerary.split(',') {
                // Parse format: ORIGIN-DEST-AIRLINE-NUMBER-DATE
                let parts: Vec<&str> = leg.split('-').collect();
                
                if parts.len() >= 4 {
                    flight_legs.push(crate::FlightLeg {
                        airline_code: parts[2].to_string(),
                        flight_number: parts[3].to_string(),
                    });
                }
            }
            
            if !flight_legs.is_empty() {
                return Some(flight_legs);
            }
        }
        
        None
    }
    
    fn extract_airport_codes(&self, item: &scraper::ElementRef) -> (Option<String>, Option<String>) {
        // Extract airport codes from span.PTuQse span[jscontroller="cNtv4b"]
        // The HTML structure contains origin and destination airport codes
        let airport_codes: Vec<String> = item.select(&self.airport_codes_selector)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        // Expect exactly 2 airport codes: [origin, destination]
        if airport_codes.len() >= 2 {
            (Some(airport_codes[0].clone()), Some(airport_codes[1].clone()))
        } else if airport_codes.len() == 1 {
            // If only one code found, we can't determine origin vs destination
            eprintln!("⚠️  Warning: Only one airport code found, expected origin and destination");
            (Some(airport_codes[0].clone()), None)
        } else {
            eprintln!("⚠️  Warning: No airport codes found in flight item");
            (None, None)
        }
    }
    
    fn extract_layover_info(&self, item: &scraper::ElementRef) -> (Option<Vec<String>>, Option<String>) {
        // Look for layover information in div.sSHqwe.tPgKwe.ogfYpf elements
        let layover_elements: Vec<_> = item.select(&self.layover_selector).collect();
        
        for element in layover_elements.iter() {
            let aria_label = element.value().attr("aria-label").unwrap_or("");
            
            // Check if this looks like layover info (contains "Layover" or "layover")
            if aria_label.to_lowercase().contains("layover") {
                // Extract description from aria-label
                let description = element.value()
                    .attr("aria-label")
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                
                // Extract airport codes from nested spans with jscontroller="cNtv4b"
                let airport_codes: Vec<String> = element.select(&self.layover_airport_selector)
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                let layovers = if airport_codes.is_empty() {
                    None
                } else {
                    Some(airport_codes)
                };
                
                return (layovers, description);
            }
        }
        
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

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

    #[test]
    fn test_extract_airport_codes() {
        let parser = FlightResponseParser::new().unwrap();
        
        // Test HTML structure with airport codes
        let html = r#"
            <li>
                <span class="PTuQse sSHqwe tPgKwe ogfYpf" aria-hidden="true">
                    <div class="QylvBf">
                        <span jscontroller="E0AZj">
                            <span jsslot="">
                                <span jscontroller="cNtv4b">LAX</span>
                            </span>
                        </span>
                    </div>
                    –
                    <div class="QylvBf">
                        <span jscontroller="E0AZj">
                            <span jsslot="">
                                <span jscontroller="cNtv4b">JFK</span>
                            </span>
                        </span>
                    </div>
                </span>
            </li>
        "#;
        
        let document = Html::parse_fragment(html);
        let li_selector = Selector::parse("li").unwrap();
        let item = document.select(&li_selector).next().unwrap();
        
        let (origin, destination) = parser.extract_airport_codes(&item);
        
        assert_eq!(origin, Some("LAX".to_string()));
        assert_eq!(destination, Some("JFK".to_string()));
    }

    #[test]
    fn test_extract_airport_codes_single_code() {
        let parser = FlightResponseParser::new().unwrap();
        
        // Test HTML structure with only one airport code
        let html = r#"
            <li>
                <span class="PTuQse sSHqwe tPgKwe ogfYpf" aria-hidden="true">
                    <div class="QylvBf">
                        <span jscontroller="E0AZj">
                            <span jsslot="">
                                <span jscontroller="cNtv4b">LAX</span>
                            </span>
                        </span>
                    </div>
                </span>
            </li>
        "#;
        
        let document = Html::parse_fragment(html);
        let li_selector = Selector::parse("li").unwrap();
        let item = document.select(&li_selector).next().unwrap();
        
        let (origin, destination) = parser.extract_airport_codes(&item);
        
        assert_eq!(origin, Some("LAX".to_string()));
        assert_eq!(destination, None);
    }

    #[test]
    fn test_extract_airport_codes_none() {
        let parser = FlightResponseParser::new().unwrap();
        
        // Test HTML structure with no airport codes
        let html = r#"
            <li>
                <div>No airport codes here</div>
            </li>
        "#;
        
        let document = Html::parse_fragment(html);
        let li_selector = Selector::parse("li").unwrap();
        let item = document.select(&li_selector).next().unwrap();
        
        let (origin, destination) = parser.extract_airport_codes(&item);
        
        assert_eq!(origin, None);
        assert_eq!(destination, None);
    }

    #[test]
    fn test_extract_layover_info() {
        let parser = FlightResponseParser::new().unwrap();
        
        // Test HTML structure with layover information
        let html = r#"
            <li>
                <div class="sSHqwe tPgKwe ogfYpf" aria-label="Layover (1 of 2) is a 2 hr 15 min layover at Paris Charles de Gaulle Airport in Paris. Layover (2 of 2) is a 4 hr 40 min overnight layover at Kempegowda International Airport Bengaluru in Bengaluru.">
                    <span jscontroller="cNtv4b">CDG</span>, 
                    <span jscontroller="cNtv4b">BLR</span>
                </div>
            </li>
        "#;
        
        let document = Html::parse_fragment(html);
        let li_selector = Selector::parse("li").unwrap();
        let item = document.select(&li_selector).next().unwrap();
        
        let (layovers, description) = parser.extract_layover_info(&item);
        
        assert_eq!(layovers, Some(vec!["CDG".to_string(), "BLR".to_string()]));
        assert_eq!(description, Some("Layover (1 of 2) is a 2 hr 15 min layover at Paris Charles de Gaulle Airport in Paris. Layover (2 of 2) is a 4 hr 40 min overnight layover at Kempegowda International Airport Bengaluru in Bengaluru.".to_string()));
    }

    #[test]
    fn test_extract_layover_info_none() {
        let parser = FlightResponseParser::new().unwrap();
        
        // Test HTML structure with no layover information
        let html = r#"
            <li>
                <div>No layover information here</div>
            </li>
        "#;
        
        let document = Html::parse_fragment(html);
        let li_selector = Selector::parse("li").unwrap();
        let item = document.select(&li_selector).next().unwrap();
        
        let (layovers, description) = parser.extract_layover_info(&item);
        
        assert_eq!(layovers, None);
        assert_eq!(description, None);
    }
} 
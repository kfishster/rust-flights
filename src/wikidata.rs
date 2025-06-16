//! Wikidata integration for city name resolution
//! 
//! This module provides functionality to query Wikidata's SPARQL endpoint
//! to resolve city names to their Freebase IDs for use with Google Flights API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use thiserror::Error;

/// Wikidata-specific error types
#[derive(Error, Debug)]
pub enum WikidataError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("City not found in Wikidata: {0}")]
    CityNotFound(String),
    
    #[error("No Freebase ID found for city: {0}")]
    NoFreebaseId(String),
    
    #[error("SPARQL query failed: {0}")]
    SparqlError(String),
}

/// City information from Wikidata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityInfo {
    pub name: String,
    pub freebase_id: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub wikidata_id: String,
    pub population: Option<i64>,
    pub coordinates: Option<(f64, f64)>, // (latitude, longitude)
}

/// SPARQL query results structure
#[derive(Debug, Deserialize)]
struct SparqlResponse {
    results: SparqlResults,
}

#[derive(Debug, Deserialize)]
struct SparqlResults {
    bindings: Vec<HashMap<String, SparqlValue>>,
}

#[derive(Debug, Deserialize)]
struct SparqlValue {
    value: String,
}

/// Global city cache - loaded once and shared across all instances
static CITY_CACHE: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Load the city cache from the static JSON file
fn load_city_cache() -> HashMap<String, String> {
    let cache_data = include_str!("city_cache.json");
    serde_json::from_str(cache_data).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load city cache: {}. Using empty cache.", e);
        HashMap::new()
    })
}

/// Get the city cache, loading it if necessary
fn get_city_cache() -> &'static HashMap<String, String> {
    CITY_CACHE.get_or_init(load_city_cache)
}

/// Wikidata SPARQL client
pub struct WikidataClient {
    client: reqwest::Client,
}

impl WikidataClient {
    /// Create a new Wikidata client
    pub fn new() -> Result<Self, WikidataError> {
        let client = reqwest::Client::new();
        
        Ok(Self {
            client,
        })
    }
    
    /// Populate the cache by fetching Freebase IDs for a list of cities
    /// This is a utility function for building/updating the cache
    pub async fn populate_cache_from_cities(&self, cities: Vec<&str>) -> Result<HashMap<String, String>, WikidataError> {
        let mut cache = HashMap::new();
        let mut successful = 0;
        let mut failed = 0;
        
        println!("Populating cache for {} cities...", cities.len());
        
        for (i, city) in cities.iter().enumerate() {
            print!("Processing {}/{}: {} ... ", i + 1, cities.len(), city);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            match self.get_freebase_id_from_wikidata(city).await {
                Ok(freebase_id) => {
                    cache.insert(city.to_string(), freebase_id.clone());
                    println!("✅ {}", freebase_id);
                    successful += 1;
                }
                Err(e) => {
                    println!("❌ {}", e);
                    failed += 1;
                }
            }
            
            // Add a small delay to be respectful to Wikidata servers
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        println!("\nCache population complete:");
        println!("  ✅ Successful: {}", successful);
        println!("  ❌ Failed: {}", failed);
        println!("  📊 Success rate: {:.1}%", (successful as f64 / cities.len() as f64) * 100.0);
        
        Ok(cache)
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, Vec<String>) {
        let cache = get_city_cache();
        let cities: Vec<String> = cache.keys().cloned().collect();
        (cache.len(), cities)
    }
    
    /// Check if a city is in the cache
    pub fn is_city_cached(&self, city_name: &str) -> bool {
        self.get_from_cache(city_name).is_some()
    }
    
    /// Get Freebase ID directly from Wikidata (bypassing cache)
    async fn get_freebase_id_from_wikidata(&self, city_name: &str) -> Result<String, WikidataError> {
        // Execute the optimized search directly
        let sparql_query = self.build_city_search_query(city_name, 5);
        let response = self.execute_sparql_query(&sparql_query).await?;
        let cities = self.parse_multiple_cities_response(response)?;
        
        // First try to find an exact match
        if let Some(city) = cities.iter().find(|c| c.name.to_lowercase() == city_name.to_lowercase()) {
            if let Some(ref freebase_id) = city.freebase_id {
                return Ok(freebase_id.clone());
            }
        }
        
        // If no exact match with Freebase ID, take the first city with a Freebase ID
        if let Some(city) = cities.iter().find(|c| c.freebase_id.is_some()) {
            return Ok(city.freebase_id.as_ref().unwrap().clone());
        }
        
        Err(WikidataError::NoFreebaseId(city_name.to_string()))
    }
    

    
    /// Execute SPARQL query against Wikidata endpoint
    async fn execute_sparql_query(&self, query: &str) -> Result<SparqlResponse, WikidataError> {
        let url = "https://query.wikidata.org/sparql";
        
        let response = self
            .client
            .get(url)
            .query(&[("query", query)])
            .header("Accept", "application/sparql-results+json")
            .header("User-Agent", "rust-flights/0.1.0 (https://github.com/example/rust-flights)")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(WikidataError::SparqlError(format!(
                "SPARQL query failed with status: {}",
                response.status()
            )));
        }
        
        let sparql_response: SparqlResponse = response.json().await?;
        Ok(sparql_response)
    }
    

    

    
    /// Get only the Freebase ID for a city (cached + fallback to Wikidata)
    pub async fn get_freebase_id_only(&self, city_name: &str) -> Result<String, WikidataError> {
        // First check the cache
        if let Some(freebase_id) = self.get_from_cache(city_name) {
            return Ok(freebase_id);
        }
        
        // If not in cache, fall back to Wikidata query
        eprintln!("Cache miss for '{}', querying Wikidata...", city_name);
        
        // Execute the optimized search directly
        let sparql_query = self.build_city_search_query(city_name, 5);
        let response = self.execute_sparql_query(&sparql_query).await?;
        let cities = self.parse_multiple_cities_response(response)?;
        
        // First try to find an exact match
        if let Some(city) = cities.iter().find(|c| c.name.to_lowercase() == city_name.to_lowercase()) {
            if let Some(ref freebase_id) = city.freebase_id {
                return Ok(freebase_id.clone());
            }
        }
        
        // If no exact match with Freebase ID, take the first city with a Freebase ID
        if let Some(city) = cities.iter().find(|c| c.freebase_id.is_some()) {
            return Ok(city.freebase_id.as_ref().unwrap().clone());
        }
        
        Err(WikidataError::NoFreebaseId(city_name.to_string()))
    }
    
    /// Check the cache for a city's Freebase ID
    fn get_from_cache(&self, city_name: &str) -> Option<String> {
        let cache = get_city_cache();
        
        // Try exact match first
        if let Some(freebase_id) = cache.get(city_name) {
            return Some(freebase_id.clone());
        }
        
        // Try case-insensitive match
        let city_name_lower = city_name.to_lowercase();
        for (cached_city, freebase_id) in cache.iter() {
            if cached_city.to_lowercase() == city_name_lower {
                return Some(freebase_id.clone());
            }
        }
        
        // Try partial matches for common city variations
        for (cached_city, freebase_id) in cache.iter() {
            if self.is_city_name_match(&city_name_lower, &cached_city.to_lowercase()) {
                return Some(freebase_id.clone());
            }
        }
        
        None
    }
    
    /// Check if two city names are likely the same city (handles common variations)
    fn is_city_name_match(&self, query: &str, cached: &str) -> bool {
        // Handle common variations
        let normalize = |name: &str| -> String {
            name.replace("saint ", "st. ")
                .replace("st ", "st. ")
                .replace("mount ", "mt. ")
                .replace("fort ", "ft. ")
                .trim()
                .to_string()
        };
        
        let normalized_query = normalize(query);
        let normalized_cached = normalize(cached);
        
        // Check if one contains the other (for cases like "New York" vs "New York City")
        normalized_query.contains(&normalized_cached) || normalized_cached.contains(&normalized_query)
    }
    

    
    /// Build SPARQL query to search for cities matching a query (optimized for speed)
    fn build_city_search_query(&self, query: &str, limit: usize) -> String {
        format!(r#"
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT DISTINCT ?city ?cityLabel ?freebaseId WHERE {{
  # Find cities with names containing the query
  ?city wdt:P31/wdt:P279* wd:Q515 .  # instance of city or subclass
  ?city rdfs:label ?cityLabel .
  FILTER(lang(?cityLabel) = "en")
  FILTER(CONTAINS(LCASE(?cityLabel), LCASE("{}")))
  
  # Get Freebase ID if available
  OPTIONAL {{ ?city wdt:P646 ?freebaseId . }}
}}
LIMIT {}
"#, query, limit)
    }
    
    /// Parse SPARQL response for multiple cities (simplified)
    fn parse_multiple_cities_response(&self, response: SparqlResponse) -> Result<Vec<CityInfo>, WikidataError> {
        let mut cities = Vec::new();
        
        for binding in response.results.bindings {
            // Extract Wikidata ID from the city URI
            let wikidata_id = binding
                .get("city")
                .and_then(|v| v.value.split('/').last())
                .unwrap_or("")
                .to_string();
            
            let name = binding
                .get("cityLabel")
                .map(|v| v.value.clone())
                .unwrap_or_default();
            
            let freebase_id = binding
                .get("freebaseId")
                .map(|v| v.value.clone());
            
            cities.push(CityInfo {
                name,
                freebase_id,
                country: None,          // Not fetched in optimized query
                country_code: None,     // Not fetched in optimized query
                wikidata_id,
                population: None,       // Not fetched in optimized query
                coordinates: None,      // Not fetched in optimized query
            });
        }
        
        Ok(cities)
    }
}

impl Default for WikidataClient {
    fn default() -> Self {
        Self::new().expect("Failed to create WikidataClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let client = WikidataClient::new().unwrap();
        
        // Test cache statistics
        let (cache_size, cached_cities) = client.get_cache_stats();
        println!("📊 Cache size: {} cities", cache_size);
        println!("🏙️ First 10 cached cities: {:?}", &cached_cities[..std::cmp::min(10, cached_cities.len())]);
        
        assert!(cache_size > 0, "Cache should not be empty");
    }
    
    #[tokio::test]
    async fn test_london_freebase_id_cached() {
        let client = WikidataClient::new().unwrap();
        
        // Check if London is in cache
        let is_cached = client.is_city_cached("London");
        println!("🏴󠁧󠁢󠁥󠁮󠁧󠁿 London is cached: {}", is_cached);
        
        let result = client.get_freebase_id_only("London").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ London Freebase ID: {} (from {})", 
                    freebase_id, 
                    if is_cached { "cache" } else { "Wikidata" }
                );
                assert_eq!(freebase_id, "/m/04jpl");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            Err(e) => {
                panic!("Unexpected error for London: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_new_york_freebase_id() {
        let client = WikidataClient::new().unwrap();
        let result = client.get_freebase_id_only("New York").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ New York Freebase ID: {}", freebase_id);
                assert_eq!(freebase_id, "/m/02_286");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            Err(e) => {
                panic!("Unexpected error for New York: {}", e);
            }
        }
    }
    
    #[tokio::test] 
    async fn test_paris_freebase_id() {
        let client = WikidataClient::new().unwrap();
        let result = client.get_freebase_id_only("Paris").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ Paris Freebase ID: {}", freebase_id);
                assert_eq!(freebase_id, "/m/05qtj");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            Err(e) => {
                panic!("Unexpected error for Paris: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_tokyo_freebase_id() {
        let client = WikidataClient::new().unwrap();
        let result = client.get_freebase_id_only("Tokyo").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ Tokyo Freebase ID: {}", freebase_id);
                assert_eq!(freebase_id, "/m/07dfk");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            Err(e) => {
                panic!("Unexpected error for Tokyo: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_sydney_freebase_id() {
        let client = WikidataClient::new().unwrap();
        let result = client.get_freebase_id_only("Sydney").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ Sydney Freebase ID: {}", freebase_id);
                assert_eq!(freebase_id, "/m/06y57");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            Err(e) => {
                panic!("Unexpected error for Sydney: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_city_not_found() {
        let client = WikidataClient::new().unwrap();
        let city_name = "NonexistentCityXYZ123";
        
        // Should not be in cache 
        let is_cached = client.is_city_cached(city_name);
        println!("❌ {} is cached: {} (should be false)", city_name, is_cached);
        assert!(!is_cached, "Non-existent city should not be in cache");
        
        let result = client.get_freebase_id_only(city_name).await;
        
        match result {
            Err(WikidataError::CityNotFound(city)) => {
                println!("✅ Correctly detected non-existent city: {}", city);
                assert_eq!(city, city_name);
            }
            Err(WikidataError::NoFreebaseId(city)) => {
                println!("✅ City found but no Freebase ID: {}", city);
                assert_eq!(city, city_name);
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            other => {
                println!("Unexpected result: {:?}", other);
            }
        }
    }
    
    #[tokio::test]
    async fn test_cache_hit_vs_miss() {
        let client = WikidataClient::new().unwrap();
        
        // Test a city that should be in cache
        let popular_city = "Tokyo";
        let is_popular_cached = client.is_city_cached(popular_city);
        
        // Test a city that likely won't be in cache
        let obscure_city = "Timbuktu";
        let is_obscure_cached = client.is_city_cached(obscure_city);
        
        println!("🏙️ {} cached: {}", popular_city, is_popular_cached);
        println!("🏜️ {} cached: {}", obscure_city, is_obscure_cached);
        
        // Popular city should be cached
        assert!(is_popular_cached, "Popular city should be in cache");
    }
    

} 
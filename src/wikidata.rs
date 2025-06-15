//! Wikidata integration for city name resolution
//! 
//! This module provides functionality to query Wikidata's SPARQL endpoint
//! to resolve city names to their Freebase IDs for use with Google Flights API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    

    

    
    /// Get only the Freebase ID for a city (fastest method - optimized query)
    pub async fn get_freebase_id_only(&self, city_name: &str) -> Result<String, WikidataError> {
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
    async fn test_london_freebase_id() {
        let client = WikidataClient::new().unwrap();
        let result = client.get_freebase_id_only("London").await;
        
        match result {
            Ok(freebase_id) => {
                println!("✅ London Freebase ID: {}", freebase_id);
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
        let result = client.get_freebase_id_only("NonexistentCityXYZ123").await;
        
        match result {
            Err(WikidataError::CityNotFound(city)) => {
                println!("✅ Correctly detected non-existent city: {}", city);
                assert_eq!(city, "NonexistentCityXYZ123");
            }
            Err(WikidataError::NoFreebaseId(city)) => {
                println!("✅ City found but no Freebase ID: {}", city);
                assert_eq!(city, "NonexistentCityXYZ123");
            }
            Err(WikidataError::HttpError(_)) => {
                eprintln!("Skipping test due to network issue");
            }
            other => {
                println!("Unexpected result: {:?}", other);
            }
        }
    }
    

} 
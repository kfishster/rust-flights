//! Benchmark Wikidata API - Test the optimized Freebase ID lookup

use rust_flights::wikidata::WikidataClient;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WikidataClient::new()?;
    let cities = vec!["London", "Paris", "New York", "Tokyo", "Sydney", "Berlin", "Madrid", "Rome"];
    
    println!("🚀 Testing optimized Wikidata Freebase ID lookup");
    println!("{}", "=".repeat(50));
    
    // Benchmark optimized Freebase ID only method
    println!("\n⚡ Getting Freebase IDs for {} cities...", cities.len());
    let start = Instant::now();
    let mut successful_lookups = 0;
    
    for city in &cities {
        match client.get_freebase_id_only(city).await {
            Ok(freebase_id) => {
                println!("  ✅ {:<10} -> {}", city, freebase_id);
                successful_lookups += 1;
            }
            Err(e) => println!("  ❌ {:<10} -> {}", city, e),
        }
    }
    
    let duration = start.elapsed();
    
    println!("\n📊 Performance Summary:");
    println!("  Total cities:      {}", cities.len());
    println!("  Successful:        {}", successful_lookups);
    println!("  Failed:            {}", cities.len() - successful_lookups);
    println!("  ⏱️  Total time:     {:?}", duration);
    println!("  ⚡ Avg per city:    {:?}", duration / cities.len() as u32);
    
    Ok(())
} 
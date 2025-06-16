use rust_flights::wikidata::WikidataClient;
use std::io::Write;

/// Top 100+ world cities by population + Top 50 US cities
const TOP_CITIES: &[&str] = &[
    // Top World Cities by Population
    "Tokyo", "Delhi", "Shanghai", "Dhaka", "São Paulo", "Cairo", "Mexico City", "Beijing",
    "Mumbai", "Osaka", "Chongqing", "Karachi", "Istanbul", "Kinshasa", "Lagos", "Buenos Aires",
    "Kolkata", "Manila", "Tianjin", "Guangzhou", "Rio de Janeiro", "Lahore", "Bangalore",
    "Shenzhen", "Moscow", "Chennai", "Bogotá", "Paris", "Jakarta", "Lima", "Bangkok", "Seoul",
    "Nagoya", "Hyderabad", "London", "Tehran", "Chicago", "Chengdu", "Nanjing", "Wuhan",
    "Ho Chi Minh City", "Luanda", "Ahmedabad", "Kuala Lumpur", "Xi'an", "Hong Kong", "Dongguan",
    "Hangzhou", "Foshan", "Shenyang", "Riyadh", "Baghdad", "Santiago", "Surat", "Madrid",
    "Suzhou", "Pune", "Harbin", "Houston", "Dallas", "Toronto", "Dar es Salaam", "Miami",
    "Belo Horizonte", "Singapore", "Philadelphia", "Atlanta", "Fukuoka", "Khartoum", "Barcelona",
    "Johannesburg", "Saint Petersburg", "Qingdao", "Dalian", "Washington", "Yangon", "Alexandria",
    "Jinan", "Guadalajara", "Boston", "Phoenix", "Melbourne", "Detroit", "Brasília", "Sydney",
    "Fortaleza", "Casablanca", "Montreal", "Izmir", "Recife", "Birmingham", "Pyongyang",
    "Faisalabad", "Porto Alegre", "Almaty", "Seattle", "Kampala", "Kyiv", "Tashkent", "Accra",
    "Addis Ababa", "Nairobi", "Bucharest", "Minsk", "Warsaw", "Budapest", "Hamburg", "Vienna",
    "Prague", "Kabul", "Algiers", "Sana'a", "Aleppo",
    
    // Major US Cities (Top 50)
    "New York", "New York City", "Los Angeles", "San Antonio", "San Diego", "San Jose", 
    "Austin", "Jacksonville", "Fort Worth", "Columbus", "Charlotte", "San Francisco", 
    "Indianapolis", "Denver", "El Paso", "Nashville", "Oklahoma City", "Portland", "Las Vegas",
    "Memphis", "Louisville", "Baltimore", "Milwaukee", "Albuquerque", "Tucson", "Fresno",
    "Sacramento", "Mesa", "Kansas City", "Long Beach", "Colorado Springs", "Raleigh",
    "Virginia Beach", "Omaha", "Oakland", "Minneapolis", "Tulsa", "Arlington", "Tampa",
    "New Orleans", "Wichita", "Cleveland", "Bakersfield", "Aurora", "Anaheim", "Honolulu",
    "Santa Ana", "Riverside", "Corpus Christi", "Lexington", "Stockton", "Henderson", "Saint Paul",
    "St. Louis", "Cincinnati", "Pittsburgh", "Greensboro", "Anchorage", "Plano", "Lincoln",
    "Orlando", "Irvine", "Newark", "Toledo", "Durham", "Chula Vista", "Fort Wayne", "Jersey City",
    "St. Petersburg", "Laredo", "Madison", "Chandler", "Buffalo", "Lubbock", "Scottsdale",
    "Reno", "Glendale", "Gilbert", "Winston-Salem", "North Las Vegas", "Norfolk", "Chesapeake",
    "Garland", "Irving", "Hialeah", "Fremont", "Boise", "Richmond", "Baton Rouge", "Spokane",
    
    // Additional Major International Cities
    "Kanpur", "Jaipur", "Lucknow", "Nagpur", "Indore", "Thane", "Bhopal", "Visakhapatnam",
    "Pimpri-Chinchwad", "Patna", "Vadodara", "Ghaziabad", "Ludhiana", "Agra", "Nashik", "Faridabad",
    "Meerut", "Rajkot", "Kalyan-Dombivli", "Vasai-Virar", "Varanasi", "Srinagar", "Aurangabad",
    "Dhanbad", "Amritsar", "Navi Mumbai", "Allahabad", "Ranchi", "Howrah", "Coimbatore", "Jabalpur",
    "Kochi", "Vijayawada", "Jodhpur", "Madurai", "Raipur", "Kota", "Guwahati", "Chandigarh",
    "Solapur", "Hubli-Dharwad", "Bareilly", "Moradabad", "Mysore", "Gurgaon", "Aligarh", "Jalandhar",
    
    // European Cities
    "Berlin", "Rome", "Naples", "Milan", "Turin", "Palermo", "Genoa", "Bologna", "Florence",
    "Bari", "Catania", "Venice", "Verona", "Messina", "Padua", "Trieste", "Taranto", "Brescia",
    "Munich", "Cologne", "Frankfurt", "Stuttgart", "Düsseldorf", "Dortmund", "Essen", "Leipzig",
    "Bremen", "Dresden", "Hannover", "Nuremberg", "Duisburg", "Bochum", "Wuppertal", "Bielefeld",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting city cache population...");
    
    let client = WikidataClient::new()?;
    
   // Deduplicate cities (case-insensitive)
   let mut unique_cities = std::collections::HashSet::new();
   let mut deduped_cities = Vec::new();
   
   for city in TOP_CITIES {
       let city_lower = city.to_lowercase();
       if unique_cities.insert(city_lower) {
           deduped_cities.push(*city);
       }
   }
   
   println!("📊 Original list: {} cities", TOP_CITIES.len());
   println!("📊 After deduplication: {} cities", deduped_cities.len());
   println!("📊 Duplicates removed: {}", TOP_CITIES.len() - deduped_cities.len());

    let cache = client.populate_cache_from_cities(deduped_cities).await?;
    
    // Write cache to JSON file
    let json_content = serde_json::to_string_pretty(&cache)?;
    let mut file = std::fs::File::create("src/city_cache.json")?;
    file.write_all(json_content.as_bytes())?;
    
    println!("\n✅ Cache written to src/city_cache.json");
    println!("📊 Total entries: {}", cache.len());
    
    // Show some example lookups
    println!("\n🔍 Testing some lookups:");
    for city in &["London", "New York", "Paris", "Tokyo", "Sydney"] {
        match client.get_freebase_id_only(city).await {
            Ok(freebase_id) => println!("  {} -> {}", city, freebase_id),
            Err(e) => println!("  {} -> Error: {}", city, e),
        }
    }
    
    Ok(())
}
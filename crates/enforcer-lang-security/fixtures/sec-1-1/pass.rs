fn connect() {
    let api_key = std::env::var("API_KEY").expect("API_KEY must be set");
    println!("{api_key}");
}

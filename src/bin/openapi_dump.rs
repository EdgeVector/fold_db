fn main() {
    let json = datafold::server::openapi::build_openapi();
    println!("{}", json);
}

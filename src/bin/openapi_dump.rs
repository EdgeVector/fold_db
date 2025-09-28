fn main() {
    let json = datafold::datafold_node::openapi::build_openapi();
    println!("{}", json);
}



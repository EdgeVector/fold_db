fn main() {
    let json = fold_db::server::openapi::build_openapi();
    println!("{}", json);
}

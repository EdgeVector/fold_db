use datafold::schema::indexing::chain_parser::ChainParser;

fn main() {
    let parser = ChainParser::new();
    
    let chain1 = parser.parse("blogpost.map().tags.split_array().map()").unwrap();
    let chain2 = parser.parse("blogpost.map().comments.map()").unwrap();
    
    println!("Chain 1: {} -> depth: {}, branch: {}", chain1.expression, chain1.depth, chain1.branch);
    println!("Chain 2: {} -> depth: {}, branch: {}", chain2.expression, chain2.depth, chain2.branch);
    
    let result = parser.analyze_compatibility(&[chain1, chain2]);
    println!("Compatibility result: {:?}", result);
    println!("Is error: {}", result.is_err());
}

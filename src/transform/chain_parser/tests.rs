#[cfg(test)]
mod tests {
    use super::super::parser::ChainParser;

    #[test]
    fn test_reducer_expression_parsing() {
        let parser = ChainParser::new();
        
        // Test parsing reducer expressions
        let expressions = vec![
            "content.count()",
            "content.split_by_word().count()",
            "content.join()",
            "content.split_by_word().join()",
        ];
        
        for expr in expressions {
            match parser.parse(expr) {
                Ok(parsed_chain) => {
                    println!("✅ Parsed '{}':", expr);
                    println!("   Operations: {:?}", parsed_chain.operations);
                    println!("   Depth: {}", parsed_chain.depth);
                    println!("   Branch: {}", parsed_chain.branch);
                }
                Err(e) => {
                    println!("❌ Failed to parse '{}': {:?}", expr, e);
                    panic!("Failed to parse reducer expression: {}", expr);
                }
            }
        }
    }
}

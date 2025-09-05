use std::env;
use std::fs;
use dot_parser_delta_computator::{parse_graph, compute_delta};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  {} parse <file.dot>", args[0]);
        eprintln!("  {} delta <file1.dot> <file2.dot>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() != 3 {
                eprintln!("Usage: {} parse <file.dot>", args[0]);
                std::process::exit(1);
            }

            let filename = &args[2];
            let content = fs::read_to_string(filename)
                .unwrap_or_else(|_| panic!("Failed to read file: {}", filename));

            match parse_graph(&content) {
                Ok(graph) => {
                    let json = serde_json::to_string_pretty(&graph)
                        .expect("Failed to serialize GraphData to JSON");
                    println!("{}", json);
                }
                Err(e) => {
                    eprintln!("Error parsing DOT file: {}", e);
                    std::process::exit(1);
                }
            }
        }

        "delta" => {
            if args.len() != 4 {
                eprintln!("Usage: {} delta <file1.dot> <file2.dot>", args[0]);
                std::process::exit(1);
            }

            let file1 = &args[2];
            let file2 = &args[3];

            let dot1 = fs::read_to_string(file1)
                .unwrap_or_else(|_| panic!("Failed to read file: {}", file1));
            let dot2 = fs::read_to_string(file2)
                .unwrap_or_else(|_| panic!("Failed to read file: {}", file2));

            let g1 = parse_graph(&dot1).expect("Failed to parse first DOT file");
            let g2 = parse_graph(&dot2).expect("Failed to parse second DOT file");

            let delta = compute_delta(&g1, &g2);
            let json = serde_json::to_string_pretty(&delta)
                .expect("Failed to serialize GraphDelta to JSON");
            println!("{}", json);
        }

        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage:");
            eprintln!("  {} parse <file.dot>", args[0]);
            eprintln!("  {} delta <file1.dot> <file2.dot>", args[0]);
            std::process::exit(1);
        }
    }
}

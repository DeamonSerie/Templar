use set_legacy_node::run;

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| "./data".into());
    let addr = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1:9000".into());
    run(&dir, &addr);
}

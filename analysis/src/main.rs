#![allow(unused)]
mod compute_robust;
mod plot;

use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: uq_project <compute|plot>");
        return Ok(());
    }

    match args[1].as_str() {
        "compute" => compute_robust::run()?,
        _ => println!("Unknown command. Use 'compute' or 'plot'."),
    }

    Ok(())
}

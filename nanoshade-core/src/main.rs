use std::path::PathBuf;
use std::time::Instant;
use clap::Parser;

mod classfile;
mod relocator;
mod treeshake;
mod jar;

use relocator::{RelocationRule, Relocator};
use jar::JarProcessor;

#[derive(Parser, Debug)]
#[command(name = "nanoshade")]
#[command(author = "NanoDev <t.me/WineSquad>")]
#[command(version = "1.0.0")]
#[command(about = "Ultra-fast native JVM bytecode relocator & tree shaker", long_about = None)]
struct Args {
    /// Входной JAR файл
    #[arg(short, long)]
    input: PathBuf,

    /// Выходной оптимизированный JAR файл
    #[arg(short, long)]
    output: PathBuf,

    /// Правила релокации пакетов: "from=to" или "from->to" (можно указывать несколько раз)
    #[arg(short, long = "relocate")]
    relocations: Vec<String>,

    /// Включить Tree Shaking (удаление неиспользуемых классов библиотек)
    #[arg(long, default_value_t = false)]
    tree_shake: bool,

    /// Точки входа для графа зависимостей (например com.example.Main)
    #[arg(long = "entry-point")]
    entry_points: Vec<String>,

    /// Префиксы пакетов/классов, которые всегда сохраняются при tree-shaking
    #[arg(long = "keep")]
    keep_prefixes: Vec<String>,
}

fn parse_relocation_rule(raw: &str) -> Option<RelocationRule> {
    let clean = raw.trim();
    if let Some((from, to)) = clean.split_once("->") {
        Some(RelocationRule::new(from.trim(), to.trim()))
    } else if let Some((from, to)) = clean.split_once('=') {
        Some(RelocationRule::new(from.trim(), to.trim()))
    } else {
        None
    }
}

fn main() {
    let args = Args::parse();
    let start_time = Instant::now();

    if !args.input.exists() {
        eprintln!("[NanoShade] Error: Input file {:?} does not exist!", args.input);
        std::process::exit(1);
    }

    let mut rules = Vec::new();
    for raw in &args.relocations {
        if let Some(rule) = parse_relocation_rule(raw) {
            rules.push(rule);
        } else {
            eprintln!("[NanoShade] Warning: Invalid relocation rule format: '{}'. Expected 'from=to' or 'from->to'", raw);
        }
    }

    println!("[NanoShade] Input:  {:?}", args.input);
    println!("[NanoShade] Output: {:?}", args.output);
    println!("[NanoShade] Active relocation rules: {}", rules.len());
    for rule in &rules {
        println!("  - {} -> {}", rule.from_dot, rule.to_dot);
    }
    println!("[NanoShade] Tree-shaking enabled: {}", args.tree_shake);

    let relocator = Relocator::new(rules);
    let processor = JarProcessor::new(
        &relocator,
        args.tree_shake,
        args.entry_points,
        args.keep_prefixes,
    );

    match processor.process(&args.input, &args.output) {
        Ok(()) => {
            let elapsed = start_time.elapsed();
            let in_size = std::fs::metadata(&args.input).map(|m| m.len()).unwrap_or(0);
            let out_size = std::fs::metadata(&args.output).map(|m| m.len()).unwrap_or(0);
            println!(
                "[NanoShade] Successfully completed in {:.2?}! (In: {:.2} MB, Out: {:.2} MB)",
                elapsed,
                in_size as f64 / 1_048_576.0,
                out_size as f64 / 1_048_576.0
            );
        }
        Err(err) => {
            eprintln!("[NanoShade] Error processing JAR: {}", err);
            std::process::exit(1);
        }
    }
}

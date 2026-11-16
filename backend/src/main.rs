use clap::{Parser, Subcommand};
use meetagain::{
    graph::RoadGraph,
    simulation::{EncounterRules, Movement, SimulationConfig, simulate},
};

#[derive(Parser)]
#[command(
    version,
    about = "MeetAgain: independent lives on Jiangsu's city road graph"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Print the weighted undirected adjacency list.
    Graph,
    /// Run an offline simulation and write full JSON to stdout.
    Simulate {
        #[arg(long, default_value_t = 20)]
        agents: u32,
        #[arg(long, default_value_t = 365)]
        ticks: u32,
        #[arg(long, default_value = "20261116")]
        seed: String,
        #[arg(long, default_value_t = 0.10)]
        home_move: f64,
        #[arg(long, default_value_t = 0.25)]
        away_move: f64,
        #[arg(long, default_value_t = 0.60)]
        home_return: f64,
        #[arg(long)]
        include_tick_zero: bool,
    },
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Graph) => {
            let graph = RoadGraph::jiangsu()?;
            println!(
                "{} nodes, {} undirected edges",
                graph.cities.len(),
                graph.roads.len()
            );
            for (i, city) in graph.cities.iter().enumerate() {
                let adjacent: Vec<_> = graph
                    .neighbors(i)
                    .iter()
                    .map(|(j, d)| format!("{} ({d:.1} km)", graph.cities[*j].name))
                    .collect();
                println!("{}: {}", city.name, adjacent.join(", "));
            }
        }
        Some(Command::Simulate {
            agents,
            ticks,
            seed,
            home_move,
            away_move,
            home_return,
            include_tick_zero,
        }) => {
            let config = SimulationConfig {
                agents,
                ticks,
                seed,
                movement: Movement {
                    home_move_probability: home_move,
                    away_move_probability: away_move,
                    home_return_probability: home_return,
                },
                encounters: EncounterRules {
                    include_tick_zero,
                    cross_tick_tolerance: 0,
                },
            };
            let result = simulate(&RoadGraph::jiangsu()?, &config)?;
            serde_json::to_writer_pretty(std::io::stdout().lock(), &result)?;
        }
        None => println!(
            "MeetAgain {} — use --help, graph, or simulate",
            env!("CARGO_PKG_VERSION")
        ),
    }
    Ok(())
}

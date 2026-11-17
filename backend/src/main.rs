use clap::{Parser, Subcommand};
use meetagain::{
    graph::RoadGraph,
    simulation::{EncounterRules, Movement, SimulationConfig, simulate},
};

#[derive(Parser)]
#[command(version, about = "MeetAgain: independent lives between cities")]
struct Cli {
    /// Directory containing map.json and the city street data.
    #[arg(long, global = true)]
    map_dir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Serve the JSON API and static frontend on one port.
    Serve {
        #[arg(long, default_value = "0.0.0.0:8080")]
        address: std::net::SocketAddr,
        #[arg(long, default_value = "frontend/dist")]
        static_dir: std::path::PathBuf,
    },
    /// Print the weighted undirected adjacency list.
    Graph,
    /// Run an offline simulation and write full JSON to stdout.
    Simulate {
        #[arg(long, default_value_t = 20)]
        agents: u32,
        #[arg(long, default_value_t = 365)]
        ticks: u32,
        /// Random seed for simulations without a separation date.
        #[arg(long, default_value = "20261116")]
        seed: String,
        /// Give every traveler the same randomly selected hometown.
        #[arg(long)]
        same_hometown: bool,
        /// Separation date (YYYY-MM-DD). Determines the random draws and festival timing.
        #[arg(long)]
        start_date: Option<String>,
        /// JSON file containing one home and graduation city choice per traveler.
        #[arg(long)]
        participants: Option<std::path::PathBuf>,
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "meetagain=info,tower_http=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let load_graph = || match &cli.map_dir {
        Some(directory) => RoadGraph::load_dir(directory),
        None => RoadGraph::bundled(),
    };
    match cli.command {
        Some(Command::Serve {
            address,
            static_dir,
        }) => meetagain::api::serve(address, static_dir, load_graph()?).await?,
        Some(Command::Graph) => {
            let graph = load_graph()?;
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
                println!(
                    "{} ({} street segments): {}",
                    city.name,
                    city.street_segments,
                    adjacent.join(", ")
                );
            }
        }
        Some(Command::Simulate {
            agents,
            ticks,
            seed,
            same_hometown,
            start_date,
            participants,
            home_move,
            away_move,
            home_return,
            include_tick_zero,
        }) => {
            let config = SimulationConfig {
                agents,
                ticks,
                seed,
                same_hometown,
                start_date,
                participants: match participants {
                    Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
                    None => Vec::new(),
                },
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
            let result = simulate(&load_graph()?, &config)?;
            serde_json::to_writer_pretty(std::io::stdout().lock(), &result)?;
        }
        None => println!(
            "MeetAgain {} — use --help, graph, or simulate",
            env!("CARGO_PKG_VERSION")
        ),
    }
    Ok(())
}

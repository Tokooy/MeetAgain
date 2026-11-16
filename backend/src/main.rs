fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("graph") {
        let graph = meetagain::graph::RoadGraph::jiangsu()?;
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
    } else {
        println!(
            "MeetAgain {} — use `graph` to inspect the network",
            env!("CARGO_PKG_VERSION")
        );
    }
    Ok(())
}

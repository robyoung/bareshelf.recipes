use std::env;
use std::path::Path;

use tantivy::{
    collector::FacetCollector, directory::MmapDirectory, query::AllQuery, schema::Facet, Index,
    TantivyError,
};

fn main() -> Result<(), TantivyError> {
    let path: String = env::args().nth(1).expect("Requires path to index");

    let index = Index::open(MmapDirectory::open(Path::new(&path))?)?;
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();

    let mut facet_collector =
        FacetCollector::for_field(index.schema().get_field("ingredient_slug").unwrap());
    facet_collector.add_facet("/ingredient");
    let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();

    let mut facets: Vec<(&Facet, u64)> = facet_counts.get("/ingredient").collect();
    facets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    for (facet, count) in facets {
        println!("{} {}", facet, count);
    }

    Ok(())
}

use std::{collections::HashSet, path::PathBuf};

use structopt::StructOpt;

use bareshelf::{searcher, Result};

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str), short, long, default_value = "./search-index")]
    path: PathBuf,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab_case")]
enum Command {
    ListIngredients,
    Search {
        #[structopt(short, long, default_value = "20")]
        limit: usize,
        facets: Vec<String>,
    },
}

fn main() -> Result<()> {
    let opt = Cli::from_args();

    let searcher = searcher(&opt.path.as_path())?;

    match opt.command {
        Command::ListIngredients => {
            for (slug, count) in searcher.recipe_ingredients()? {
                println!("{} {}", slug, count);
            }
        }
        Command::Search {
            limit,
            facets: search_facets,
        } => {
            searcher
                .recipes_by_ingredients(&search_facets, limit)?
                .iter()
                .for_each(|recipe| {
                    println!("\n> {}    ({})", recipe.recipe.title, recipe.score);
                    println!(
                        "{} matching, {} missing",
                        recipe.recipe.ingredients.len() - recipe.missing_ingredients.len(),
                        recipe.missing_ingredients.len()
                    );
                    println!("Missing: {:?}", recipe.missing_ingredients);
                    let missing_set: HashSet<_> =
                        recipe.missing_ingredients.iter().cloned().collect();
                    for ingredient in &recipe.recipe.ingredients {
                        print!("    - {}", ingredient.slug);
                        if missing_set.contains(&ingredient.slug) {
                            print!("  - MISSING");
                        }
                        println!();
                    }
                });
        }
    }

    Ok(())
}

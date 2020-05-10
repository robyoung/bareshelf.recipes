use std::{collections::HashSet, path::PathBuf};

use structopt::StructOpt;

use bareshelf::{searcher, IngredientQuery, RecipeQuery, Result};

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
    IngredientsByPrefix {
        prefix: String,
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
            let query = RecipeQuery::default()
                .shelf_ingredients(&search_facets)
                .limit(limit);

            searcher.recipes(query)?.all().iter().for_each(|recipe| {
                println!("\n> {}    ({})", recipe.recipe.title, recipe.score);
                println!(
                    "{} matching, {} missing",
                    recipe.recipe.ingredients.len() - recipe.missing_ingredients.len(),
                    recipe.missing_ingredients.len()
                );
                println!("Missing: {:?}", recipe.missing_ingredients);
                let missing_set: HashSet<_> = recipe.missing_ingredients.iter().cloned().collect();
                for ingredient in &recipe.recipe.ingredients {
                    print!("    - {}", ingredient.slug);
                    if missing_set.contains(&ingredient.slug) {
                        print!("  - MISSING");
                    }
                    println!();
                }
            });
        }
        Command::IngredientsByPrefix { prefix } => {
            searcher
                .ingredients(IngredientQuery::by_prefix(&prefix))?
                .iter()
                .for_each(|ingredient| {
                    println!("{:?}", ingredient);
                });
        }
    }

    Ok(())
}

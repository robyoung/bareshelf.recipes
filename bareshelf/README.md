# Search

```
use search;

let index = search::create_or_open("path/to/base");

// indexing
let indexer = search.indexer();

let ingredients = vec![];
let recipes = vec![];

recipes.iter().for_each(indexer.add_recipe);
ingrediends.iter().for_each(indexer.add_ingredient);
indexer.commit();

// searching
let ingredient_search = search.ingredients();
ingredient_search.all();
ingredient_search.by_prefix("butt");


let recipe_search = search.recipes();
recipe_search.all();                // facet counts and top n
recipe_search.search(ingredients);  // ingredient slugs
```

```
let searcher = searcher(path);

searcher.recipes_by_ingredients(ingredients);
searcher.ingredients_by_prefix(prefix);



```

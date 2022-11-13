import os

from flask import current_app

from . import bareshelf_indexer
from .database import db
from .models import Ingredient, Recipe


def index() -> None:
    path = current_app.config["SEARCH_INDEX_PATH"]
    if not os.path.exists(path):
        os.mkdir(path)
    index = bareshelf_indexer.create_or_open(path)

    print("Indexing ingredients...", flush=True, end="")
    for ingredient in db.session.query(Ingredient):
        doc = bareshelf_indexer.Ingredient(ingredient.name, ingredient.slug)
        index.add_ingredient(doc)
    print("DONE")

    print("Indexing recipes...", flush=True, end="")
    for recipe in db.session.query(Recipe):
        doc = bareshelf_indexer.Recipe(recipe.title, recipe.slug, recipe.url)

        if recipe.chef_name:
            doc.chef_name = recipe.chef_name
        if recipe.image_name:
            doc.image_name = recipe.image_name

        for ingredient in recipe.ingredients:
            if ingredient.ingredient is not None:
                doc.add_ingredient(
                    ingredient.ingredient.name, ingredient.ingredient.slug
                )

        index.add_recipe(doc)
    print("DONE")

    print("Committing...", flush=True, end="")
    index.commit()
    print("DONE")

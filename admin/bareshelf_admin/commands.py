import os

from flask import current_app

from . import indexer
from .database import db
from .models import Recipe


def index():
    path = current_app.config["SEARCH_INDEX_PATH"]
    if not os.path.exists(path):
        os.mkdir(path)
    index = indexer.create_or_open(path)

    for recipe in db.session.query(Recipe):
        doc = indexer.Recipe(recipe.title, recipe.slug)
        for ingredient in recipe.ingredients:
            if ingredient.ingredient is not None:
                doc.add_ingredient(ingredient.ingredient.name, ingredient.ingredient.slug)
        index.add(doc)

    index.commit()

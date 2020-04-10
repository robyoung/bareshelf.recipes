import os

from . import search
from .database import db
from .models import Recipe


def index():
    path = "./search_index"
    if not os.path.exists(path):
        os.mkdir(path)
    index = search.create_or_open(path)

    for recipe in db.session.query(Recipe):
        doc = search.Recipe(recipe.title, recipe.slug)
        for ingredient in recipe.ingredients:
            if ingredient.ingredient is not None:
                doc.add_ingredient(ingredient.ingredient.name, ingredient.ingredient.slug)
        index.add(doc)

    index.commit()

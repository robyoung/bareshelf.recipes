"""Add initial tags

Tag all ingredients used in more than 100 recipes

Revision ID: fc8694aff96f
Revises: 5ccbad865613
Create Date: 2020-04-27 18:40:05.297422

"""
from typing import List

from alembic import op


# revision identifiers, used by Alembic.
revision = "fc8694aff96f"
down_revision = "5ccbad865613"
branch_labels = None
depends_on = None


def upgrade():
    _add_tag("Spices", "spices")
    spices = [
        "black-pepper",
        "chilli",
        "dried-chilli",
        "chilli-powder",
        "cinnamon",
        "ginger",
        "cumin",
        "coriander",
        "nutmeg",
        "turmeric",
        "cloves",
        "paprika",
        "cardamom",
        "ground-ginger",
        "cayenne-pepper",
        "star-anise",
        "vanilla-pod",
        "vanilla-extract",
        "coriander-seeds",
        "curry-powder",
    ]
    _add_ingredients_to_tag("spices", spices)

    _add_tag("Herbs", "herbs")
    herbs = [
        "parsley",
        "oregano",
        "thyme",
        "bay-leaf",
        "fresh-coriander",
        "mint",
        "basil",
        "chives",
        "rosemary",
        "tarragon",
        "sage",
        "dill",
        "fennel",
        "chervil",
    ]
    _add_ingredients_to_tag("herbs", herbs)

    _add_tag("Nuts & Seeds", "nuts-seeds")
    nuts_seeds = [
        "sesame-seeds",
        "pine-nut",
        "ground-almonds",
        "flaked-almonds",
        "walnut",
    ]
    _add_ingredients_to_tag("nuts-seeds", nuts_seeds)

    _add_tag("Basics", "basics")
    basics = [
        "oil",
        "olive-oil",
        "vegetable-oil",
        "sunflower-oil",
        "sesame-oil",
        "rapeseed-oil",
        "pepper",
        "black-pepper",
        "egg",
        "egg-yolk",
        "egg-white",
        "salt",
        "sea-salt",
        "garlic",
        "stock",
        "chicken-stock",
        "beef-stock",
        "vegetable-stock",
        "honey",
        "sugar",
        "cornflour",
        "tomato-purée",
        "soy-sauce",
        "dijon-mustard",
        "bread",
        "breadcrumbs",
        "worcestershire-sauce",
        "vinegar",
        "balsamic-vinegar",
        "white-wine-vinegar",
        "red-wine-vinegar",
        "white-wine",
        "red-wine",
        # baking
        "flour",
        "self-raising-flour",
        "plain-flour",
        "baking-powder",
        "caster-sugar",
        "icing-sugar",
        "brown-sugar",
        "dark-chocolate",
        "puff-pastry",
        "cocoa-powder",
    ]
    _add_ingredients_to_tag("basics", basics)

    # maybe this one??
    # _add_tag('Baking', 'baking')
    # _add_ingredients_to_tag('baking', [
    # ])

    _add_tag("Fruit", "fruit")
    fruit = [
        "lemon",
        "tomato",
        "lime",
        "lemon-juice",
        "orange",
        "apple",
    ]
    _add_ingredients_to_tag("fruit", fruit)

    _add_tag("Vegetables", "vegetables")
    vegetables = [
        "onion",
        "spring-onion",
        "red-onion",
        "shallot",
        "carrot",
        "broccoli",
        "aubergine",
        "potato",
        "new-potatoes",
        "spinach",
        "celery",
        "leek",
        "peas",
        "chopped-tomatoes",
        "cherry-tomatoes",
        "mushroom",
        "courgette",
        "cucumber",
        "watercress",
        "asparagus",
        "sweet-potato",
        "olive",
        "capers",
    ]
    _add_ingredients_to_tag("vegetables", vegetables)

    _add_tag("Fish", "fish")
    fish = [
        "salmon",
    ]
    _add_ingredients_to_tag("fish", fish)

    _add_tag("Seafood", "seafood")
    seafood = [
        "prawn",
    ]
    _add_ingredients_to_tag("seafood", seafood)

    _add_tag("Meat", "meat")
    meat = [
        "chicken",
        "chicken-breast",
        "bacon",
        "pancetta",
    ]
    _add_ingredients_to_tag("meat", meat)

    _add_tag("Dairy", "dairy")
    dairy = [
        "butter",
        "cheddar",
        "double-cream",
        "milk",
        "parmesan",
        "yoghurt",
        "crème-fraîche",
        "cream-cheese",
    ]
    _add_ingredients_to_tag("dairy", dairy)

    _add_tag("Top ingredients", "top")
    _add_ingredients_to_tag(
        "top",
        list(
            set(
                spices
                + herbs
                + nuts_seeds
                + basics
                + fruit
                + vegetables
                + fish
                + seafood
                + meat
                + dairy
            )
        ),
    )


def _add_tag(name: str, slug: str) -> None:
    op.execute(f"INSERT INTO tags (name, slug) VALUES ('{name}', '{slug}')")


def _add_ingredients_to_tag(tag_slug: str, ingredient_slugs: List[str]) -> None:
    tag_id = _get_tag_id(tag_slug)
    ingredient_ids = _get_ingredient_ids(ingredient_slugs)
    assert len(ingredient_ids) == len(
        ingredient_slugs
    ), f"not all ingredients found {ingredient_slugs} ({len(ingredient_slugs)} != {len(ingredient_ids)}"
    op.execute(
        "INSERT INTO ingredient_tags (tag_id, ingredient_id) VALUES {}".format(
            ", ".join(
                f"({tag_id}, {ingredient_id})" for ingredient_id in ingredient_ids
            )
        )
    )
    pass


def _get_tag_id(slug: str) -> int:
    conn = op.get_bind()
    return conn.execute(f"SELECT id FROM tags WHERE slug='{slug}'").first().id


def _get_ingredient_ids(slugs: List[str]) -> List[int]:
    conn = op.get_bind()
    query = "SELECT id, slug FROM ingredients WHERE slug IN ({})".format(
        ", ".join(f"'{slug}'" for slug in slugs)
    )

    ingredients = [ingredient for ingredient in conn.execute(query).fetchall()]

    if len(ingredients) != len(slugs):
        missing = set(slugs) - set(ingredient.slug for ingredient in ingredients)
        raise AssertionError(
            f"not all ingredients found: ({len(ingredients)} != {len(slugs)}) {missing}"
        )

    return [ingredient.id for ingredient in ingredients]


def downgrade():
    op.execute("DELETE FROM ingredient_tags")
    op.execute("DELETE FROM tags")

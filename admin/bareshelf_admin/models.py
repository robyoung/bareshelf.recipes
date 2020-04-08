from typing import Any, Optional

from sqlalchemy import event

from .database import db


class Ingredient(db.Model):  # type: ignore
    __tablename__ = "ingredient"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=False)

    slug = db.Column(db.String, nullable=False, unique=True, index=True)
    url = db.Column(db.String, nullable=False, unique=True, index=True)

    # TODO: maybe just do this with namespaced slug like pasta/fusilli
    parent_id = db.Column(db.Integer, db.ForeignKey("ingredient.id"), nullable=True)

    recipes = db.relationship("Recipe", secondary="recipe_ingredient", lazy="subquery",)
    recipe_ingredients = db.relationship(
        "RecipeIngredient", back_populates="ingredient"
    )

    @staticmethod
    def get_by_url(url: str) -> Optional['Ingredient']:
        return db.session.query(Ingredient).filter(Ingredient.url == url).first()

    def __repr__(self) -> str:
        return f'<Ingredient "{self.name}">'


class QuantityUnit(db.Model):  # type: ignore
    __tablename__ = "quantity_unit"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=True)
    abbreviation = db.Column(db.String, nullable=True)


class Recipe(db.Model):  # type: ignore
    __tablename__ = "recipe"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    title = db.Column(db.String, nullable=False, unique=False)
    slug = db.Column(db.String, nullable=False, unique=True, index=True)
    url = db.Column(db.String, nullable=False, unique=True, index=True)

    ingredients = db.relationship("RecipeIngredient", back_populates="recipe")

    @staticmethod
    def get_by_url(url: str) -> Optional['Recipe']:
        return db.session.query(Recipe).filter(Recipe.url == url).first()

    def __repr__(self) -> str:
        return f'<Recipe "{self.title}">'


class RecipeIngredient(db.Model):  # type: ignore
    __tablename__ = "recipe_ingredient"

    ingredient_id = db.Column(
        db.Integer, db.ForeignKey("ingredient.id"), primary_key=True
    )
    recipe_id = db.Column(db.Integer, db.ForeignKey("recipe.id"), primary_key=True)
    quantity_unit_id = db.Column(
        db.Integer, db.ForeignKey("quantity_unit.id"), nullable=True
    )
    quantity = db.Column(db.Numeric, nullable=True)
    description = db.Column(db.String, nullable=False)

    recipe = db.relationship("Recipe", back_populates="ingredients")
    ingredient = db.relationship("Ingredient", back_populates="recipe_ingredients")
    quantity_unit = db.relationship("QuantityUnit")


def auto_slug(field: Any) -> None:
    @event.listens_for(field, "set")  # type: ignore
    def fn(target: Any, value: str, initiator: Any, event: Any) -> None:
        setattr(target, "slug", value.lower().replace(" ", "-"))


auto_slug(Ingredient.name)
auto_slug(Recipe.title)

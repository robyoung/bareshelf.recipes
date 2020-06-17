from typing import Any, Optional
from datetime import datetime

from sqlalchemy import event

from .database import db


class WithSlug:
    slug = db.Column(db.String, nullable=False, unique=True, index=True)

    @classmethod
    def get_by_slug(cls, slug: str) -> Optional[Any]:
        return db.session.query(cls).filter(cls.slug == slug).first()


class WithURL:
    url = db.Column(db.String, nullable=False, unique=True, index=True)

    @classmethod
    def get_by_url(cls, url: str) -> Optional[Any]:
        return db.session.query(cls).filter(cls.url == url).first()


class Timestamps:
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(
        db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow
    )


ingredient_tags = db.Table(
    "ingredient_tags",
    db.Column("tag_id", db.Integer, db.ForeignKey("tags.id"), primary_key=True),
    db.Column(
        "ingredient_id", db.Integer, db.ForeignKey("ingredients.id"), primary_key=True
    ),
)


class Ingredient(db.Model, WithSlug, WithURL, Timestamps):  # type: ignore
    __tablename__ = "ingredients"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=False)

    parent_id = db.Column(db.Integer, db.ForeignKey("ingredients.id"), nullable=True)

    recipes = db.relationship(
        "Recipe", secondary="recipe_ingredients", lazy="subquery",
    )
    recipe_ingredients = db.relationship(
        "RecipeIngredient", back_populates="ingredient"
    )

    tags = db.relationship(
        "Tag",
        secondary=ingredient_tags,
        lazy="subquery",
        backref=db.backref("ingredients", lazy=True),
    )

    def __repr__(self) -> str:
        return f'<Ingredient "{self.name}">'


class Tag(db.Model, WithSlug, Timestamps):
    __tablename__ = "tags"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=False)


class QuantityUnit(db.Model, Timestamps):  # type: ignore
    __tablename__ = "quantity_units"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=True)
    abbreviation = db.Column(db.String, nullable=True)


class Recipe(db.Model, WithSlug, WithURL, Timestamps):  # type: ignore
    __tablename__ = "recipes"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    title = db.Column(db.String, nullable=False, unique=False)

    chef_name = db.Column(db.String, nullable=True)
    image_name = db.Column(db.String, nullable=True)

    ingredients = db.relationship(
        "RecipeIngredient", back_populates="recipe", lazy="joined"
    )

    def __repr__(self) -> str:
        return f'<Recipe "{self.title}">'


class RecipeIngredient(db.Model, Timestamps):  # type: ignore
    __tablename__ = "recipe_ingredients"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    recipe_id = db.Column(db.Integer, db.ForeignKey("recipes.id"), nullable=False)
    ingredient_id = db.Column(
        db.Integer, db.ForeignKey("ingredients.id"), nullable=True
    )
    quantity_unit_id = db.Column(
        db.Integer, db.ForeignKey("quantity_units.id"), nullable=True
    )
    quantity = db.Column(db.Numeric, nullable=True)
    description = db.Column(db.String, nullable=False)

    recipe = db.relationship("Recipe", back_populates="ingredients")
    ingredient = db.relationship(
        "Ingredient", back_populates="recipe_ingredients", lazy="joined"
    )
    quantity_unit = db.relationship("QuantityUnit")


def auto_slug(field: Any) -> None:
    @event.listens_for(field, "set")  # type: ignore
    def fn(target: Any, value: str, initiator: Any, event: Any) -> None:
        setattr(target, "slug", value.lower().replace(" ", "-"))


auto_slug(Ingredient.name)
auto_slug(Recipe.title)

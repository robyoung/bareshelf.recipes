import os

from flask import Flask
from flask_admin import Admin
from flask_admin.contrib.sqla import ModelView
from flask_sqlalchemy import SQLAlchemy

app = Flask(__name__)

# set optional bootswatch theme
app.config["FLASK_ADMIN_SWATCH"] = "flatly"
app.config["SQLALCHEMY_DATABASE_URI"] = "sqlite:////tmp/test.db"
app.config["SECRET_KEY"] = os.environ["ADMIN_SECRET_KEY"]

admin = Admin(app, name="bareshelf.recipe", template_mode="bootstrap3")
db = SQLAlchemy(app)

recipe_ingredients = db.Table(
    "recipe_ingredient",
    db.Column("recipe_id", db.Integer, db.ForeignKey("recipe.id"), primary_key=True),
    db.Column(
        "ingredient_id", db.Integer, db.ForeignKey("ingredient.id"), primary_key=True
    ),
)


class Ingredient(db.Model):
    __tablename__ = "ingredient"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    name = db.Column(db.String, nullable=False, unique=True)
    # TODO: is there a plugin for slugs?
    slug = db.Column(db.String, nullable=False, unique=True, index=True)

    # TODO: maybe just do this with namespaced slug like pasta/fusilli
    parent_id = db.Column(db.Integer, db.ForeignKey("ingredient.id"), nullable=True)

    def __repr__(self) -> str:
        return f'<Ingredient "{self.name}">'


class Recipe(db.Model):
    __tablename__ = "recipe"

    id = db.Column(db.Integer, primary_key=True, autoincrement=True)
    title = db.Column(db.String, nullable=False, unique=False)
    # TODO: is there a plugin for slugs?
    slug = db.Column(db.String, nullable=False, unique=True, index=True)

    ingredients = db.relationship(
        "Ingredient",
        secondary=recipe_ingredients,
        lazy="subquery",
        backref=db.backref("recipes", lazy=True),
    )

    def __repr__(self) -> str:
        return f'<Recipe "{self.title}">'


# Add administrative views here
admin.add_view(ModelView(Ingredient, db.session))
admin.add_view(ModelView(Recipe, db.session))


app.run()

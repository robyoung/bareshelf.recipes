import os

from flask import Flask
from flask_admin import Admin
from flask_admin.contrib.sqla import ModelView
from flask_migrate import Migrate

from .database import db
from .views import IngredientView, RecipeView
from .models import Ingredient, Recipe, RecipeIngredient


admin = Admin(name="bareshelf.recipe", template_mode="bootstrap3")


def create_app():
    app = Flask(__name__)

    # set optional bootswatch theme
    app.config["FLASK_ADMIN_SWATCH"] = "flatly"
    app.config["SQLALCHEMY_DATABASE_URI"] = os.environ["ADMIN_SQLALCHEMY_DATABASE_URL"]
    app.config["SQLALCHEMY_TRACK_MODIFICATIONS"] = False
    app.config["SECRET_KEY"] = os.environ["ADMIN_SECRET_KEY"]

    db.init_app(app)
    app.migrate = Migrate(app, db)

    admin.init_app(app)

    admin.add_view(IngredientView(Ingredient, db.session))
    admin.add_view(RecipeView(Recipe, db.session))
    admin.add_view(ModelView(RecipeIngredient, db.session))

    return app

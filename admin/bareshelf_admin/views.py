from flask_admin.contrib.sqla import ModelView

from .models import RecipeIngredient


class IngredientView(ModelView):
    form_excluded_columns = ["slug"]


class RecipeView(ModelView):
    form_excluded_columns = ["slug"]
    inline_models = (RecipeIngredient,)

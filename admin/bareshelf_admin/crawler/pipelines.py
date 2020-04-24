# -*- coding: utf-8 -*-

# Define your item pipelines here
#
# Don't forget to add your pipeline to the ITEM_PIPELINES setting
# See: https://docs.scrapy.org/en/latest/topics/item-pipeline.html
from typing import Optional, Any, Mapping

import scrapy

from bareshelf_admin.application import create_app
from bareshelf_admin.database import db
from bareshelf_admin.models import Recipe, Ingredient, RecipeIngredient


class SQLAlchemyPipeline:
    def __init__(self) -> None:
        self.app = create_app()

    def process_item(self, item: Mapping, spider: scrapy.Spider) -> Optional[Mapping]:
        with self.app.app_context():  # type: ignore
            if hasattr(spider, "model"):
                if not hasattr(spider.model, "get_by_url"):
                    raise ValueError(
                        f"Invalid model {spider.model}, must have get_by_url"
                    )

                self.process_model(spider.model, item)

                db.session.commit()

            return item

    def process_model(self, model: Any, item: Mapping[str, Any]) -> Any:
        instance = model.get_by_url(item["url"])
        if model == Recipe:
            return self.recipe_model(model, instance, item)
        else:
            return self.basic_model(model, instance, item)

    def basic_model(
        self, model: Any, instance: Optional[Any], item: Mapping[str, Any]
    ) -> Any:
        if instance:
            for key, value in item.items():
                setattr(instance, key, value)
        else:
            instance = model(**item)
            for i in range(2, 10):
                original_slug = instance.slug
                if model.get_by_slug(instance.slug):
                    instance.slug = f"{original_slug}-{i}"
                else:
                    break

        db.session.add(instance)

        return instance

    def recipe_model(
        self, model: Any, instance: Optional[Any], item: Mapping[str, Any]
    ) -> Any:
        recipe_item = dict(item)
        if item["images"]:
            recipe_item["image_name"] = item["images"][0]["path"].split("/")[1]
        recipe_item.pop("images", None)
        recipe_item.pop("image_urls", None)
        ingredients = recipe_item.pop("ingredients")

        instance = self.basic_model(model, instance, recipe_item)

        lookup = {ingredient["url"]: ingredient for ingredient in ingredients}

        for ingredient in instance.ingredients:
            lookup_item = lookup.pop(ingredient.ingredient.url, None)
            if lookup_item is None:
                db.session.delete(ingredient)
            else:
                ingredient.ingredient = Ingredient.get_by_url(lookup_item["url"])
                ingredient.description = lookup_item["description"]

        for ingredient in lookup.values():
            db.session.add(
                RecipeIngredient(
                    recipe=instance,
                    ingredient=Ingredient.get_by_url(ingredient["url"]),
                    description=ingredient["description"],
                )
            )

        return instance

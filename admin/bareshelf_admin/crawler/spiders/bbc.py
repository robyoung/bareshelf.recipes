from typing import Any, Iterable, Union, Mapping

import scrapy
from scrapy.http import Response, Request

from bareshelf_admin.models import Ingredient, Recipe


class BBCPaginationMixin:
    def follow_pages(self, response: Response) -> Iterable[Request]:
        yield from (
            response.follow(letter.get(), callback=self.parse)  # type: ignore
            for letter in response.css(".az-keyboard ul li a::attr(href)")
        )
        yield from (
            response.follow(number.get(), callback=self.parse)  # type: ignore
            for number in response.css("ul.pagination__list li a::attr(href)")
        )


class BBCIngredientsSpider(scrapy.Spider, BBCPaginationMixin):  # type: ignore
    name = "bbc-ingredients"
    model = Ingredient
    start_urls = ("https://www.bbc.co.uk/food/ingredients/a-z",)

    def parse(self, response: Response) -> Iterable[Union[Request, Mapping]]:
        yield from self.follow_pages(response)
        for ingredient in response.css("a.promo__ingredient"):
            yield {
                "name": ingredient.css("h3::text").get(),
                "url": response.urljoin(ingredient.attrib["href"]),
            }


class BBCRecipeSpider(scrapy.Spider, BBCPaginationMixin):  # type: ignore
    name = "bbc-recipes"
    model = Recipe
    start_urls = ("https://www.bbc.co.uk/food/recipes/a-z",)

    def parse(self, response: Response) -> Iterable[Union[Request, Mapping]]:
        yield from self.follow_pages(response)
        for recipe_url in response.css("a.promo::attr(href)"):
            yield response.follow(recipe_url.get(), callback=self.parse)

        recipe = response.css("div.recipe-main-info")
        if recipe:
            ingredients = [
                self._get_ingredient(response, ingredient)
                for ingredient in recipe.css("li.recipe-ingredients__list-item")
            ]
            if all(ingredient["url"] is not None for ingredient in ingredients):
                chef_name_parts = recipe.css(".chef__name *::text").getall()
                chef_name = chef_name_parts[-1] if len(chef_name_parts) > 0 else None
                image_urls = recipe.css(".recipe-media__image img::attr(src)").getall()
                yield {
                    "title": recipe.css("h1::text").get(),
                    "url": response.url,
                    "chef_name": chef_name,
                    "ingredients": ingredients,
                    "image_urls": image_urls,
                }

    def _get_ingredient(self, response: Response, ingredient: Any) -> Mapping:
        url = ingredient.css("a::attr(href)").get()
        if url is not None:
            url = response.urljoin(url)
        description = "".join(ingredient.css("::text").getall())

        result = {
            "description": description,
            "url": url,
        }

        return result

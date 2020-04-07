import scrapy

from bareshelf_admin.models import Ingredient

class BBCIngredientsSpider(scrapy.Spider):
    name = "bbc-ingredients"
    model = Ingredient

    def start_requests(self):
        urls = (
            "https://www.bbc.co.uk/food/ingredients/a-z",
        )
        yield from (scrapy.Request(url=url, callback=self.parse) for url in urls)

    def parse(self, response):
        yield from (
            scrapy.Request(response.urljoin(letter.get()), callback=self.parse)
            for letter in response.css(".az-keyboard ul li a::attr(href)")
        )
        yield from (
            scrapy.Request(response.urljoin(number.get()), callback=self.parse)
            for number in response.css("ul.pagination__list li a::attr(href)")
        )
        for ingredient in response.css("a.promo__ingredient"):
            yield {
                "name": ingredient.css("h3::text").get(),
                "url": response.urljoin(ingredient.attrib["href"]),
            }

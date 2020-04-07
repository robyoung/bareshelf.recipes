# -*- coding: utf-8 -*-

# Define your item pipelines here
#
# Don't forget to add your pipeline to the ITEM_PIPELINES setting
# See: https://docs.scrapy.org/en/latest/topics/item-pipeline.html
from bareshelf_admin.application import create_app
from bareshelf_admin.database import db


class CrawlerPipeline(object):
    def process_item(self, item, spider):
        return item


class SQLAlchemyPipeline:
    def __init__(self):
        self.app = create_app()
    
    def process_item(self, item, spider):
        with self.app.app_context():
            if hasattr(spider, "model"):
                if not hasattr(spider.model, "get_by_url"):
                    raise ValueError(f"Invalid model {spider.model}, must have get_by_url")

                instance = spider.model.get_by_url(item["url"])
                if instance:
                    for key, value in item.items():
                        setattr(instance, key, value)
                else:
                    instance = spider.model(**item)
                
                db.session.add(instance)
                db.session.commit()

            return item

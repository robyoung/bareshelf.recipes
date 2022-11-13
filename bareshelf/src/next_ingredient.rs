//! Next ingredient collector
//!
//! This collector will show which single ingredients will open you up to the
//! largest number of new recipes.
//!
//! TODO: FIx the name, it's aweful but I can't think of anything better right now.
use std::{
    collections::{HashMap, HashSet},
    u64,
};
use tantivy::{
    collector::{Collector, SegmentCollector},
    fastfield::FacetReader,
    schema::{Facet, Field},
    DocId, Result, Score, SegmentOrdinal, SegmentReader, TantivyError,
};

pub(crate) struct NextIngredientCollector {
    field: Field,
    shelf: Vec<Facet>,
}

pub(crate) struct NextIngredientSegmentCollector {
    reader: FacetReader,
    shelf: HashSet<u64>,
    counts: HashMap<u64, usize>,
    facet_ords_buf: Vec<u64>,
}

impl NextIngredientCollector {
    pub fn new(field: Field, shelf: Vec<Facet>) -> Self {
        Self { field, shelf }
    }
}

impl Collector for NextIngredientCollector {
    type Fruit = HashMap<Facet, usize>;
    type Child = NextIngredientSegmentCollector;

    fn for_segment(
        &self,
        _: SegmentOrdinal,
        reader: &SegmentReader,
    ) -> Result<NextIngredientSegmentCollector> {
        let field_name = reader.schema().get_field_name(self.field);
        let facet_reader = reader.facet_reader(self.field).map_err(|_| {
            TantivyError::SchemaError(format!("Field {:?} is not a facet field.", field_name))
        })?;
        let facet_dict = facet_reader.facet_dict();
        let shelf = self
            .shelf
            .iter()
            .filter_map(|key| {
                facet_dict
                    .term_ord(key.encoded_str())
                    .expect("IO error here implies the index is borked")
            })
            .collect::<HashSet<_>>();

        Ok(NextIngredientSegmentCollector {
            reader: facet_reader,
            shelf,
            counts: HashMap::new(),
            facet_ords_buf: Vec::with_capacity(255),
        })
    }

    fn merge_fruits(&self, segments_facet_counts: Vec<Self::Fruit>) -> Result<Self::Fruit> {
        let mut facet_counts = HashMap::new();
        for segment_facet_counts in segments_facet_counts {
            for (facet, count) in segment_facet_counts {
                *(facet_counts.entry(facet).or_insert(0)) += count;
            }
        }
        Ok(facet_counts)
    }

    fn requires_scoring(&self) -> bool {
        false
    }
}

impl SegmentCollector for NextIngredientSegmentCollector {
    type Fruit = HashMap<Facet, usize>;

    fn collect(&mut self, doc: DocId, _: Score) {
        self.reader.facet_ords(doc, &mut self.facet_ords_buf);
        let missing: Vec<u64> = self
            .facet_ords_buf
            .iter()
            .filter(|o| !self.shelf.contains(o))
            .cloned()
            .collect();
        if missing.len() == 1 {
            let counter = self.counts.entry(missing[0]).or_insert(0);
            *counter += 1;
        }
    }

    fn harvest(self) -> Self::Fruit {
        let facet_dict = self.reader.facet_dict();
        self.counts
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(ord, count)| {
                let mut facet = vec![];
                facet_dict
                    .ord_to_term(*ord, &mut facet)
                    .expect("IO error here implies the index is borked");
                (Facet::from_encoded(facet).unwrap(), *count)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::{
        doc,
        query::AllQuery,
        schema::{Schema, TEXT},
        Index,
    };

    #[test]
    fn next_ingredient_collector() {
        let mut schema_builder = Schema::builder();

        let name = schema_builder.add_text_field("name", TEXT);
        let ingredient = schema_builder.add_facet_field("ingredient", ());

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);

        let mut writer = index.writer(30_000__000).unwrap();

        writer
            .add_document(doc!(
                name => "Scrambled Egg",
                ingredient => Facet::from("/ingredient/egg"),
                ingredient => Facet::from("/ingredient/butter"),
                ingredient => Facet::from("/ingredient/salt"),
            ))
            .unwrap();
        writer
            .add_document(doc!(
                name => "Potato Wedges",
                ingredient => Facet::from("/ingredient/potato"),
                ingredient => Facet::from("/ingredient/oil"),
                ingredient => Facet::from("/ingredient/salt"),
            ))
            .unwrap();
        writer
            .add_document(doc!(
                name => "Jam on Toast",
                ingredient => Facet::from("/ingredient/bread"),
                ingredient => Facet::from("/ingredient/butter"),
                ingredient => Facet::from("/ingredient/jam"),
            ))
            .unwrap();
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();

        let collector = NextIngredientCollector::new(
            ingredient,
            vec![
                Facet::from("/ingredient/bread"),
                Facet::from("/ingredient/butter"),
                Facet::from("/ingredient/egg"),
                Facet::from("/ingredient/potato"),
                Facet::from("/ingredient/oil"),
            ],
        );
        let mut facet_counts: Vec<(Facet, usize)> = searcher
            .search(&AllQuery, &collector)
            .unwrap()
            .into_iter()
            .collect();

        facet_counts.sort_unstable();

        assert_eq!(
            facet_counts,
            vec![
                (Facet::from("/ingredient/jam"), 1),
                (Facet::from("/ingredient/salt"), 2)
            ]
        );
    }
}

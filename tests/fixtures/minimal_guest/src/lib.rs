use cauld_ron::guest::wit::{GeneratedFile, Guest, export};

struct FixtureGuest;

impl Guest for FixtureGuest {
    fn build() -> Vec<GeneratedFile> {
        vec![GeneratedFile {
            path: "example/test.ron".to_string(),
            ron_text: "(name:\"fixture\",count:3)\n".to_string(),
        }]
    }
}

export!(FixtureGuest with_types_in cauld_ron::guest);

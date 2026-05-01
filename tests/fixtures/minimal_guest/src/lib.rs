wit_bindgen::generate!({
    path: "../../../wit",
    world: "content-module",
});

struct FixtureGuest;

impl Guest for FixtureGuest {
    fn build() -> Vec<cauld_ron::build::types::GeneratedFile> {
        vec![cauld_ron::build::types::GeneratedFile {
            path: "example/test.ron".to_string(),
            ron_text: "(name:\"fixture\",count:3)\n".to_string(),
        }]
    }
}

export!(FixtureGuest);

wit_bindgen::generate!({
    path: "../../../wit",
    world: "content-module",
});

struct FixtureGuest;

impl Guest for FixtureGuest {
    fn build() -> Vec<vessel::build::types::GeneratedFile> {
        vec![vessel::build::types::GeneratedFile {
            path: "example/test.ron".to_string(),
            ron_text: "(name:\"fixture\",count:3)\n".to_string(),
        }]
    }
}

export!(FixtureGuest);

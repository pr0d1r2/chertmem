use super::{Ref, candidates};

const ID: &str = "01JB2X3Y4Z5A6B7C8D9E0F1G2H";

#[test]
fn a_ref_shows_as_chert_colon() {
    let r = Ref::stored("01JB2X3Y".to_owned());
    assert_eq!(
        (r.to_string(), r.as_str()),
        ("chert:01JB2X3Y".into(), "01JB2X3Y")
    );
}

#[test]
fn the_full_id_is_always_a_candidate() {
    assert_eq!(candidates(ID).last(), Some(ID));
}

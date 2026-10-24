use super::State;

#[test]
fn states_round_trip_through_their_names() {
    for s in [State::Hot, State::Warm, State::Cold] {
        assert_eq!(State::parse(s.as_str()), Some(s));
    }
    assert_eq!(State::Warm.as_str(), "warm");
    assert_eq!(State::parse("lukewarm"), None);
}

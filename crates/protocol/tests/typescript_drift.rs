#[test]
fn checked_in_typescript_matches_rust_protocol() {
    let checked_in = include_str!("../../../packages/shared/src/protocol.generated.ts");
    assert_eq!(checked_in, protocol::typescript_declarations());
}

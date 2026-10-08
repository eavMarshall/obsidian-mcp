use pulldown_cmark::Parser;
#[tokio::test]
async fn test_links() {
    let md = "Normal [[link1]]";
    for event in Parser::new(md) {
        println!("{:?}", event);
    }
}

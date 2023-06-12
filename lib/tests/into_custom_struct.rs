use neo4rs::*;

mod container;

#[derive(Debug, FromBoltType, PartialEq)]
struct Post {
    title: String,
    body: Option<String>,
}

#[derive(Debug, FromBoltType, PartialEq)]
struct Unit;

#[tokio::test]
async fn from_row_for_custom_struct() {
    let config = ConfigBuilder::default()
        .db("neo4j")
        .fetch_size(500)
        .max_connections(10);
    let neo4j = container::Neo4jContainer::from_config(config).await;
    let graph = neo4j.graph();

    let mut result = graph
        .execute(
            query("RETURN { title: $title, body: $body } as n")
                .params([("title", "Hello"), ("body", "World!")]),
        )
        .await
        .unwrap();
    let row = result.next().await.unwrap().unwrap();
    let value: Post = row.get("n").unwrap();
    assert_eq!(
        Post {
            title: "Hello".into(),
            body: Some("World!".into()),
        },
        value
    );
    assert!(result.next().await.unwrap().is_none());

    let mut result = graph
        .execute(
            query("RETURN { title: $title } as n")
                .params([("title", "With empty body")]),
        )
        .await
        .unwrap();
    let row = result.next().await.unwrap().unwrap();
    let value: Post = row.get("n").unwrap();
    assert_eq!(
        Post {
            title: "With empty body".into(),
            body: None,
        },
        value
    );
    assert!(result.next().await.unwrap().is_none());
}

#[tokio::test]
async fn from_node_props_for_custom_struct() {
    let config = ConfigBuilder::default()
        .db("neo4j")
        .fetch_size(500)
        .max_connections(10);
    let neo4j = container::Neo4jContainer::from_config(config).await;
    let graph = neo4j.graph();

    let mut result = graph
        .execute(
            query("CREATE (n:Post { title: $title, body: $body }) RETURN n")
                .params([("title", "Hello"), ("body", "World!")]),
        )
        .await
        .unwrap();

    while let Ok(Some(row)) = result.next().await {
        let node: Node = row.get("n").unwrap();
        let id = node.id();
        let labels = node.labels();

        let value: Post = node.props().unwrap();

        assert_eq!(
            Post {
                title: "Hello".into(),
                body: Some("World!".into()),
            },
            value
        );
        assert_eq!(labels, vec!["Post"]);
        assert!(id >= 0);
    }
}

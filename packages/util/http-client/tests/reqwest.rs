use http_client::{Client, codec, header_policy, transport};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, header_exists, method},
};

#[derive(
    Debug, PartialEq, serde::Serialize, serde::Deserialize, minicbor::Encode, minicbor::Decode,
)]
struct Foo {
    #[n(0)]
    x: u32,
}

#[tokio::test]
async fn native_json_roundtrip() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&Foo { x: 42 }))
        .mount(&server)
        .await;
    let client = Client::new(transport::Reqwest::new(None), codec::Json, server.uri());
    let resp: Foo = client.get("/foo").await.unwrap();
    assert_eq!(resp, Foo { x: 42 });
}

#[tokio::test]
async fn native_cbor_roundtrip() {
    let server = MockServer::start().await;
    let body = minicbor::to_vec(&Foo { x: 42 }).unwrap();
    Mock::given(method("GET"))
        .and(header("Accept", "application/cbor"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/cbor"))
        .mount(&server)
        .await;
    let client = Client::new(transport::Reqwest::new(None), codec::Cbor, server.uri());
    let resp: Foo = client.get("/foo").await.unwrap();
    assert_eq!(resp, Foo { x: 42 });
}

#[tokio::test]
async fn native_json_and_cbor_equivalent() {
    let server = MockServer::start().await;

    let foo = Foo { x: 42 };
    let cbor_body = minicbor::to_vec(&foo).unwrap();

    Mock::given(method("GET"))
        .and(header("Accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&foo))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(header("Accept", "application/cbor"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(cbor_body, "application/cbor"))
        .mount(&server)
        .await;

    let json_client = Client::new(transport::Reqwest::new(None), codec::Json, server.uri());
    let cbor_client = Client::new(transport::Reqwest::new(None), codec::Cbor, server.uri());

    let json_resp: Foo = json_client.get("/foo").await.unwrap();
    let cbor_resp: Foo = cbor_client.get("/foo").await.unwrap();

    assert_eq!(json_resp, cbor_resp);
}

/// Policy sets Content-Type and Content-Length on POST with a body.
#[tokio::test]
async fn policy_sets_content_headers_on_post() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("Content-Type", "application/json"))
        .and(header_exists("Content-Length"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&Foo { x: 1 }))
        .mount(&server)
        .await;

    let client = Client::new(transport::Reqwest::new(None), codec::Json, server.uri());

    let resp: Foo = client.post("/foo", &Foo { x: 1 }).await.unwrap();

    assert_eq!(resp, Foo { x: 1 });
}

/// Policy sets Accept (not Content-Type) on GET with no body.
#[tokio::test]
async fn policy_sets_accept_on_get() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("Accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&Foo { x: 7 }))
        .mount(&server)
        .await;
    let client = Client::new(transport::Reqwest::new(None), codec::Json, server.uri());
    let resp: Foo = client.get("/foo").await.unwrap();
    assert_eq!(resp, Foo { x: 7 });
}

/// send_no_policy skips all client policies; manually set header is used instead.
#[tokio::test]
async fn send_no_policy_skips_client_policies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("Accept", "application/cbor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&Foo { x: 3 }))
        .mount(&server)
        .await;
    // Client has json policy, but we override entirely via send_no_policy + map_builder.
    let client = Client::new(transport::Reqwest::new(None), codec::Json, server.uri());
    let resp: Foo = client
        .get_with_headers(
            "/foo",
            vec![header_policy::Accept::from_decoder::<()>(&codec::Cbor).boxed()],
        )
        .await
        .unwrap();
    assert_eq!(resp, Foo { x: 3 });
}

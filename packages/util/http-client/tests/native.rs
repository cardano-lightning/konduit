// tests/native.r// tests/native.rs
use http_client::{CborCodec, HttpClient, JsonCodec, ReqwestTransport, header_policy};
use wiremock::matchers::{header, header_exists, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    let client = HttpClient::new(ReqwestTransport::new(None), JsonCodec, server.uri());
    let resp: Foo = client
        .request(http::Method::GET, "/foo")
        .send::<(), Foo>(None)
        .await
        .unwrap();
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
    let client = HttpClient::new(ReqwestTransport::new(None), CborCodec, server.uri());
    let resp: Foo = client
        .request(http::Method::GET, "/foo")
        .map_builder(|b| b.header("ACCEPT", "application/cbor"))
        .send::<(), Foo>(None)
        .await
        .unwrap();
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
    let json_client = HttpClient::new(ReqwestTransport::new(None), JsonCodec, server.uri())
        .with_policy(header_policy::ContentNegotiation::from_encoder(&JsonCodec));
    let cbor_client = HttpClient::new(ReqwestTransport::new(None), CborCodec, server.uri())
        .with_policy(header_policy::ContentNegotiation::from_encoder(&CborCodec));
    let json_resp: Foo = json_client
        .request(http::Method::GET, "/foo")
        .send::<(), Foo>(None)
        .await
        .unwrap();
    let cbor_resp: Foo = cbor_client
        .request(http::Method::GET, "/foo")
        .send::<(), Foo>(None)
        .await
        .unwrap();
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
    let client = HttpClient::new(ReqwestTransport::new(None), JsonCodec, server.uri())
        .with_policy(header_policy::ContentNegotiation::from_encoder(&JsonCodec))
        .with_policy(header_policy::ContentLength);
    let resp: Foo = client
        .request(http::Method::POST, "/foo")
        .send::<Foo, Foo>(Some(&Foo { x: 1 }))
        .await
        .unwrap();
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
    let client = HttpClient::new(ReqwestTransport::new(None), JsonCodec, server.uri())
        .with_policy(header_policy::ContentNegotiation::from_encoder(&JsonCodec));
    let resp: Foo = client
        .request(http::Method::GET, "/foo")
        .send::<(), Foo>(None)
        .await
        .unwrap();
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
    let client = HttpClient::new(ReqwestTransport::new(None), JsonCodec, server.uri())
        .with_policy(header_policy::ContentNegotiation::from_encoder(&JsonCodec));
    let resp: Foo = client
        .request(http::Method::GET, "/foo")
        .map_builder(|b| b.header("Accept", "application/cbor"))
        .send_no_policy::<(), Foo>(None)
        .await
        .unwrap();
    assert_eq!(resp, Foo { x: 3 });
}

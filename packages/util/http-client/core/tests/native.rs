// tests/native.rs
use http_client::{CborCodec, HttpClient, JsonCodec, NativeTransport};
use wiremock::matchers::{header, method};
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
    eprintln!("server.uri(): {}", server.uri());
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&Foo { x: 42 }))
        .mount(&server)
        .await;

    let client = HttpClient::new(NativeTransport::new(None), JsonCodec, server.uri());

    let resp: Result<Foo, _> = client
        .request(http::Method::GET, "/foo")
        .send::<(), Foo>(None)
        .await;

    eprintln!(
        "received {} requests",
        server.received_requests().await.unwrap().len()
    );
    let received = server.received_requests().await.unwrap();
    for req in &received {
        eprintln!("method: {}", req.method);
        eprintln!("url: {}", req.url);
        eprintln!("headers: {:#?}", req.headers);
    }

    let _resp = resp.unwrap();
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

    let client = HttpClient::new(NativeTransport::new(None), CborCodec, server.uri());

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
        .respond_with(ResponseTemplate::new(200).set_body_bytes(cbor_body))
        .mount(&server)
        .await;

    let json_client = HttpClient::new(NativeTransport::new(None), JsonCodec, server.uri());
    let cbor_client = HttpClient::new(NativeTransport::new(None), CborCodec, server.uri());

    let json_resp: Foo = json_client
        .request(http::Method::GET, "/foo")
        .map_builder(|b| b.header("ACCEPT", "application/json"))
        .send::<(), Foo>(None)
        .await
        .unwrap();
    let cbor_resp: Foo = cbor_client
        .request(http::Method::GET, "/foo")
        .map_builder(|b| b.header("ACCEPT", "application/cbor"))
        .send::<(), Foo>(None)
        .await
        .unwrap();

    assert_eq!(json_resp, cbor_resp);
}

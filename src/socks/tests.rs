use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

use super::negotiate;

#[tokio::test]
async fn negotiate_accepts_noauth_when_it_is_the_only_method() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        negotiate(&mut stream).await
    });

    let client = async move {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();

        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await.unwrap();
        response
    };

    let response = timeout(Duration::from_secs(2), client)
        .await
        .expect("client timed out");

    assert_eq!(response, [0x05, 0x00]);

    timeout(Duration::from_secs(2), server)
        .await
        .expect("server timed out")
        .expect("server task panicked")
        .expect("negotiation failed");
}

// Exercise the complete handler so that rejection also verifies connection closure.
async fn expect_method_rejection(greeting: &[u8]) {
    timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let server = tokio::spawn(super::socksv5_handle(stream));

        client.write_all(greeting).await.unwrap();
        let mut response = [0; 2];
        client.read_exact(&mut response).await.unwrap();
        assert_eq!(response, [0x05, 0xff]);
        assert_eq!(client.read(&mut [0; 1]).await.unwrap(), 0);
        server.await.unwrap();
    })
    .await
    .expect("rejection did not finish");
}

#[tokio::test]
async fn rejects_unsupported_authentication_methods() {
    expect_method_rejection(&[0x05, 0x02, 0x01, 0x02]).await;
}

#[tokio::test]
async fn rejects_empty_authentication_methods() {
    expect_method_rejection(&[0x05, 0x00]).await;
}

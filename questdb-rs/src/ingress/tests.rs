/*******************************************************************************
 *     ___                  _   ____  ____
 *    / _ \ _   _  ___  ___| |_|  _ \| __ )
 *   | | | | | | |/ _ \/ __| __| | | |  _ \
 *   | |_| | |_| |  __/\__ \ |_| |_| | |_) |
 *    \__\_\\__,_|\___||___/\__|____/|____/
 *
 *  Copyright (c) 2014-2019 Appsicle
 *  Copyright (c) 2019-2025 QuestDB
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *
 ******************************************************************************/

use super::*;
#[cfg(feature = "sync-sender-http")]
use crate::tests::mock::{HttpResponse, MockServer};
use crate::ErrorCode;

#[cfg(feature = "sync-sender-tcp")]
use tempfile::TempDir;

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_simple() {
    let builder = SenderBuilder::from_conf("http::addr=127.0.0.1;").unwrap();
    assert_eq!(builder.protocol, Protocol::Http);
    assert_specified_endpoints_eq(
        &builder.endpoints,
        vec![("127.0.0.1", Protocol::Http.default_port())],
    );
    assert!(!builder.protocol.tls_enabled());
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn https_simple() {
    let builder = SenderBuilder::from_conf("https::addr=localhost;").unwrap();
    assert_eq!(builder.protocol, Protocol::Https);
    assert_specified_endpoints_eq(
        &builder.endpoints,
        vec![("localhost", Protocol::Https.default_port())],
    );
    assert!(builder.protocol.tls_enabled());

    #[cfg(feature = "tls-webpki-certs")]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::WebpkiRoots);

    #[cfg(not(feature = "tls-webpki-certs"))]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::OsRoots);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcp_simple() {
    let builder = SenderBuilder::from_conf("tcp::addr=127.0.0.1;").unwrap();
    assert_eq!(builder.protocol, Protocol::Tcp);
    assert_specified_endpoints_eq(
        &builder.endpoints,
        vec![("127.0.0.1", Protocol::Tcp.default_port())],
    );
    assert!(!builder.protocol.tls_enabled());
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_simple() {
    let builder = SenderBuilder::from_conf("tcps::addr=localhost;").unwrap();
    assert_eq!(builder.protocol, Protocol::Tcps);
    assert_specified_endpoints_eq(
        &builder.endpoints,
        vec![("localhost", Protocol::Tcps.default_port())],
    );
    assert!(builder.protocol.tls_enabled());

    #[cfg(feature = "tls-webpki-certs")]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::WebpkiRoots);

    #[cfg(not(feature = "tls-webpki-certs"))]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::OsRoots);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn invalid_value() {
    assert_conf_err(
        SenderBuilder::from_conf("tcp::addr=localhost\n;"),
        "Config parse error: invalid char '\\n' in value at position 19",
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn specified_cant_change() {
    let mut builder = SenderBuilder::from_conf("tcp::addr=localhost;").unwrap();
    builder = builder.bind_interface("1.1.1.1").unwrap();
    assert_conf_err(
        builder.bind_interface("1.1.1.2"),
        "\"bind_interface\" is already specified",
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn missing_addr() {
    assert_conf_err(
        SenderBuilder::from_conf("tcp::"),
        "Missing \"addr\" parameter in config string",
    );
}

#[cfg(any(feature = "sync-sender-tcp", feature = "sync-sender-http"))]
#[test]
fn unsupported_service() {
    assert_conf_err(
        SenderBuilder::from_conf("xaxa::addr=localhost;"),
        "Unsupported protocol: xaxa",
    );
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_basic_auth() {
    let builder =
        SenderBuilder::from_conf("http::addr=localhost;username=user123;password=pass321;")
            .unwrap();
    let auth = builder.build_auth().unwrap();
    match auth.unwrap() {
        conf::AuthParams::Basic(conf::BasicAuthParams { username, password }) => {
            assert_eq!(username, "user123");
            assert_eq!(password, "pass321");
        }
        _ => {
            panic!("Expected AuthParams::Basic");
        }
    }
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_token_auth() {
    let builder = SenderBuilder::from_conf("http::addr=localhost:9000;token=token123;").unwrap();
    let auth = builder.build_auth().unwrap();
    match auth.unwrap() {
        conf::AuthParams::Token(conf::TokenAuthParams { token }) => {
            assert_eq!(token, "token123");
        }
        _ => {
            panic!("Expected AuthParams::Token");
        }
    }
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn incomplete_basic_auth() {
    assert_conf_err(
        SenderBuilder::from_conf("http::addr=localhost;username=user123;")
            .unwrap()
            .build(),
        "Basic authentication parameter \"username\" is present, but \"password\" is missing.",
    );
    assert_conf_err(
        SenderBuilder::from_conf("http::addr=localhost;password=pass321;")
            .unwrap()
            .build(),
        "Basic authentication parameter \"password\" is present, but \"username\" is missing.",
    );
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn zero_timeout_forbidden() {
    assert_conf_err(
        SenderBuilder::from_conf("http::addr=localhost;username=user123;request_timeout=0;"),
        "\"request_timeout\" must be greater than 0.",
    );

    assert_conf_err(
        SenderBuilder::new(Protocol::Http, "localhost", 9000)
            .request_timeout(Duration::from_millis(0)),
        "\"request_timeout\" must be greater than 0.",
    );
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn misspelled_basic_auth() {
    assert_conf_err(
        Sender::from_conf("http::addr=localhost;username=user123;pass=pass321;"),
        r##"Basic authentication parameter "username" is present, but "password" is missing."##,
    );
    assert_conf_err(
        Sender::from_conf("http::addr=localhost;user=user123;password=pass321;"),
        r##"Basic authentication parameter "password" is present, but "username" is missing."##,
    );
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn inconsistent_http_auth() {
    let expected_err_msg = r##"Inconsistent HTTP authentication parameters. Specify either "username" and "password", or just "token"."##;
    assert_conf_err(
        Sender::from_conf("http::addr=localhost;username=user123;token=token123;"),
        expected_err_msg,
    );
    assert_conf_err(
        Sender::from_conf("http::addr=localhost;password=pass321;token=token123;"),
        expected_err_msg,
    );
}

#[cfg(all(feature = "sync-sender-tcp", feature = "sync-sender-http"))]
#[test]
fn cant_use_basic_auth_with_tcp() {
    let builder = SenderBuilder::new(Protocol::Tcp, "localhost", 9000)
        .username("user123")
        .unwrap()
        .password("pass321")
        .unwrap();
    assert_conf_err(
        builder.build_auth(),
        "The \"basic_auth\" setting can only be used with the ILP/HTTP protocol.",
    );
}

#[cfg(all(feature = "sync-sender-tcp", feature = "sync-sender-http"))]
#[test]
fn cant_use_token_auth_with_tcp() {
    let builder = SenderBuilder::new(Protocol::Tcp, "localhost", 9000)
        .token("token123")
        .unwrap();
    assert_conf_err(
        builder.build_auth(),
        "Token authentication only be used with the ILP/HTTP protocol.",
    );
}

#[cfg(all(feature = "sync-sender-tcp", feature = "sync-sender-http"))]
#[test]
fn cant_use_ecdsa_auth_with_http() {
    let builder = SenderBuilder::from_conf("http::addr=localhost;")
        .unwrap()
        .username("key_id123")
        .unwrap()
        .token("priv_key123")
        .unwrap()
        .token_x("pub_key1")
        .unwrap()
        .token_y("pub_key2")
        .unwrap();
    assert_conf_err(
        builder.build_auth(),
        "ECDSA authentication is only available with ILP/TCP and not available with ILP/HTTP.",
    );
}

#[cfg(all(not(feature = "sync-sender-tcp"), feature = "sync-sender-http"))]
#[test]
fn cant_use_ecdsa_auth_with_http_ex_tcp_support() {
    let mk_builder = || {
        SenderBuilder::from_conf("http::addr=localhost;")
            .unwrap()
            .username("key_id123")
            .unwrap()
            .token("priv_key123")
            .unwrap()
    };

    assert_conf_err(
        mk_builder().token_x("pub_key1"),
        "cannot specify \"token_x\": ECDSA authentication is only available with ILP/TCP and not available with ILP/HTTP.",
    );

    assert_conf_err(
        mk_builder().token_y("pub_key2"),
        "cannot specify \"token_y\": ECDSA authentication is only available with ILP/TCP and not available with ILP/HTTP.",
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn set_auth_specifies_tcp() {
    let mut builder = SenderBuilder::new(Protocol::Tcp, "localhost", 9000);
    assert_eq!(builder.protocol, Protocol::Tcp);
    builder = builder
        .username("key_id123")
        .unwrap()
        .token("priv_key123")
        .unwrap()
        .token_x("pub_key1")
        .unwrap()
        .token_y("pub_key2")
        .unwrap();
    assert_eq!(builder.protocol, Protocol::Tcp);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn set_net_interface_specifies_tcp() {
    let builder = SenderBuilder::new(Protocol::Tcp, "localhost", 9000);
    assert_eq!(builder.protocol, Protocol::Tcp);
    builder.bind_interface("55.88.0.4").unwrap();
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcp_ecdsa_auth() {
    let builder = SenderBuilder::from_conf(
        "tcp::addr=localhost:9000;username=user123;token=token123;token_x=xtok123;token_y=ytok123;",
    )
    .unwrap();
    let auth = builder.build_auth().unwrap();
    match auth.unwrap() {
        conf::AuthParams::Ecdsa(conf::EcdsaAuthParams {
            key_id,
            priv_key,
            pub_key_x,
            pub_key_y,
        }) => {
            assert_eq!(key_id, "user123");
            assert_eq!(priv_key, "token123");
            assert_eq!(pub_key_x, "xtok123");
            assert_eq!(pub_key_y, "ytok123");
        }
        #[cfg(feature = "sync-sender-http")]
        _ => {
            panic!("Expected AuthParams::Ecdsa");
        }
    }
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn incomplete_tcp_ecdsa_auth() {
    let expected_err_msg = r##"Incomplete ECDSA authentication parameters. Specify either all or none of: "username", "token", "token_x", "token_y"."##;
    assert_conf_err(
        SenderBuilder::from_conf("tcp::addr=localhost;username=user123;")
            .unwrap()
            .build(),
        expected_err_msg,
    );
    assert_conf_err(
        SenderBuilder::from_conf("tcp::addr=localhost;username=user123;token=token123;")
            .unwrap()
            .build(),
        expected_err_msg,
    );
    assert_conf_err(
        SenderBuilder::from_conf(
            "tcp::addr=localhost;username=user123;token=token123;token_x=123;",
        )
        .unwrap()
        .build(),
        expected_err_msg,
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn misspelled_tcp_ecdsa_auth() {
    assert_conf_err(
        Sender::from_conf("tcp::addr=localhost;username=user123;tokenx=123;"),
        "Incomplete ECDSA authentication parameters. Specify either all or none of: \"username\", \"token\", \"token_x\", \"token_y\"."
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_verify_on() {
    let builder = SenderBuilder::from_conf("tcps::addr=localhost;tls_verify=on;").unwrap();
    assert!(builder.protocol.tls_enabled());

    #[cfg(feature = "tls-webpki-certs")]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::WebpkiRoots);

    #[cfg(not(feature = "tls-webpki-certs"))]
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::OsRoots);
}

#[cfg(feature = "sync-sender-tcp")]
#[cfg(feature = "insecure-skip-verify")]
#[test]
fn tcps_tls_verify_unsafe_off() {
    let builder = SenderBuilder::from_conf("tcps::addr=localhost;tls_verify=unsafe_off;").unwrap();
    assert!(builder.protocol.tls_enabled());
    assert_defaulted_eq(&builder.tls_ca, CertificateAuthority::WebpkiRoots);
    assert_specified_eq(&builder.tls_verify, false);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_verify_invalid() {
    assert_conf_err(
        SenderBuilder::from_conf("tcps::addr=localhost;tls_verify=off;"),
        r##"Config parameter "tls_verify" must be either "on" or "unsafe_off".'"##,
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_roots_webpki() {
    let builder = SenderBuilder::from_conf("tcps::addr=localhost;tls_ca=webpki_roots;");

    #[cfg(feature = "tls-webpki-certs")]
    {
        let builder = builder.unwrap();
        assert!(builder.protocol.tls_enabled());
        assert_specified_eq(&builder.tls_ca, CertificateAuthority::WebpkiRoots);
        assert_defaulted_eq(&builder.tls_roots, None);
    }

    #[cfg(not(feature = "tls-webpki-certs"))]
    assert_eq!(
        "Config parameter \"tls_ca=webpki_roots\" requires the \"tls-webpki-certs\" feature",
        builder.unwrap_err().msg()
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[cfg(feature = "tls-native-certs")]
#[test]
fn tcps_tls_roots_os() {
    let builder = SenderBuilder::from_conf("tcps::addr=localhost;tls_ca=os_roots;").unwrap();
    assert!(builder.protocol.tls_enabled());
    assert_specified_eq(&builder.tls_ca, CertificateAuthority::OsRoots);
    assert_defaulted_eq(&builder.tls_roots, None);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_roots_file() {
    use std::io::Write;

    // Write a dummy file to test the file path
    let tmp_dir = TempDir::new().unwrap();
    let path = tmp_dir.path().join("cacerts.pem");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(b"dummy").unwrap();
    let builder = SenderBuilder::from_conf(format!(
        "tcps::addr=localhost;tls_roots={};",
        path.to_str().unwrap()
    ))
    .unwrap();
    assert_specified_eq(&builder.tls_ca, CertificateAuthority::PemFile);
    assert_specified_eq(&builder.tls_roots, path);
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_roots_file_missing() {
    let err =
        SenderBuilder::from_conf("tcps::addr=localhost;tls_roots=/some/invalid/path/cacerts.pem;")
            .unwrap_err();
    assert_eq!(err.code(), ErrorCode::ConfigError);
    assert!(err
        .msg()
        .contains("Could not open root certificate file from path"));
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn tcps_tls_roots_file_with_password() {
    use std::io::Write;

    let tmp_dir = TempDir::new().unwrap();
    let path = tmp_dir.path().join("cacerts.pem");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(b"dummy").unwrap();
    let builder_or_err = SenderBuilder::from_conf(format!(
        "tcps::addr=localhost;tls_roots={};tls_roots_password=extremely_secure;",
        path.to_str().unwrap()
    ));
    assert_conf_err(builder_or_err, "\"tls_roots_password\" is not supported.");
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_request_min_throughput() {
    let builder =
        SenderBuilder::from_conf("http::addr=localhost;request_min_throughput=100;").unwrap();
    let Some(http_config) = builder.http else {
        panic!("Expected Some(HttpConfig)");
    };
    assert_specified_eq(&http_config.request_min_throughput, 100u64);
    assert_defaulted_eq(&http_config.request_timeout, Duration::from_millis(10000));
    assert_defaulted_eq(&http_config.retry_timeout, Duration::from_millis(10000));
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_request_timeout() {
    let builder = SenderBuilder::from_conf("http::addr=localhost;request_timeout=100;").unwrap();
    let Some(http_config) = builder.http else {
        panic!("Expected Some(HttpConfig)");
    };
    assert_defaulted_eq(&http_config.request_min_throughput, 102400u64);
    assert_specified_eq(&http_config.request_timeout, Duration::from_millis(100));
    assert_defaulted_eq(&http_config.retry_timeout, Duration::from_millis(10000));
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn http_retry_timeout() {
    let builder = SenderBuilder::from_conf("http::addr=localhost;retry_timeout=100;").unwrap();
    let Some(http_config) = builder.http else {
        panic!("Expected Some(HttpConfig)");
    };
    assert_defaulted_eq(&http_config.request_min_throughput, 102400u64);
    assert_defaulted_eq(&http_config.request_timeout, Duration::from_millis(10000));
    assert_specified_eq(&http_config.retry_timeout, Duration::from_millis(100));
}

#[cfg(feature = "sync-sender-http")]
mod multi_endpoint_http_tests {
    use std::io;

    use super::*;

    #[derive(PartialEq, Eq)]
    struct WriteStatusCode {
        code: u16,
        text: String,
        body: String,
    }

    #[derive(PartialEq, Eq)]
    enum WriteState {
        NotAccepting,
        Accepting,
        Error(WriteStatusCode),
    }

    struct ServerConfig {
        expect_settings: bool,
        expect_write: WriteState,
    }

    impl Default for ServerConfig {
        fn default() -> Self {
            Self {
                expect_settings: false,
                expect_write: WriteState::NotAccepting,
            }
        }
    }

    fn spawn_server(server: MockServer, cfg: ServerConfig) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut srv = server;
            let accepting_writes = cfg.expect_write != WriteState::NotAccepting;
            if cfg.expect_settings || accepting_writes {
                srv.accept().unwrap();
            }

            if cfg.expect_settings {
                let req = srv.recv_http_q().unwrap();
                assert_eq!(req.method(), "GET");
                assert!(req.path().ends_with("/settings"));
                srv.send_settings_response().unwrap();
            }

            if accepting_writes {
                loop {
                    match srv.recv_http(0.5) {
                        Ok(req) => {
                            assert_eq!(req.method(), "POST");
                            assert!(req.path().starts_with("/write"));
                            let resp = if let WriteState::Error(ref status_code) = cfg.expect_write
                            {
                                HttpResponse::empty()
                                    .with_status(status_code.code, &status_code.text)
                                    .with_body_str(&status_code.body)
                            } else {
                                HttpResponse::empty()
                            };
                            srv.send_http_response_q(resp).unwrap();
                        }
                        Err(err) if err.kind() == io::ErrorKind::TimedOut => break,
                        Err(err) => {
                            panic!("Receiving http: {err:?}")
                        }
                    }
                }
            }
        })
    }

    fn build_sender(endpoints: Vec<(&str, u16)>) -> Sender {
        SenderBuilder::new_multi_endpoints(Protocol::Http, endpoints)
            .request_min_throughput(0)
            .unwrap()
            .request_timeout(Duration::from_millis(100))
            .unwrap()
            .retry_timeout(Duration::from_millis(300))
            .unwrap()
            .build()
            .unwrap()
    }

    #[test]
    fn version_auto_v2() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 130);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1], 130);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                ..Default::default()
            },
        );
        let t2 = spawn_server(s2, ServerConfig::default());

        let sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        // autodetecting only based on the first endpoint in config
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn version_auto_v1() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1], 110);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 100);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                ..Default::default()
            },
        );
        let t2 = spawn_server(s2, ServerConfig::default());

        let sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        // autodetecting only based on the first endpoint in config
        assert_eq!(sender.protocol_version(), ProtocolVersion::V1);

        t1.join().unwrap();
        t2.join().unwrap();
    }

    fn flush(sender: &mut Sender) -> Result<()> {
        let mut buf = sender.new_buffer();
        buf.table("x")
            .unwrap()
            .symbol("x", "x")
            .unwrap()
            .at_now()
            .unwrap();
        sender.flush(&mut buf)
    }

    #[test]
    fn respect_config_order() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 120);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );
        let t2 = spawn_server(s2, ServerConfig::default());

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn check_accepting() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 120);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 404,
                    text: "Not Found".to_string(),
                    body: "".to_string(),
                }),
                ..Default::default()
            },
        );
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn failover_on_404() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 404,
                    text: "Not Found".to_string(),
                    body: "".to_string(),
                }),
            },
        );
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn failover_on_421() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 421,
                    text: "Misdirected Request".to_string(),
                    body: "".to_string(),
                }),
            },
        );
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn failover_on_conn_err() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(s1, ServerConfig::default());
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn no_failover_on_500() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 500,
                    text: "Internal Server Error".to_string(),
                    body: "Maybe one more time?".to_string(),
                }),
                ..Default::default()
            },
        );
        let t2 = spawn_server(s2, ServerConfig::default());

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        let flush_res = flush(&mut sender);
        assert!(flush_res.is_err());
        let flush_err = flush_res.err().unwrap();
        assert_eq!(
            flush_err.msg(),
            format!("Could not flush buffer: Maybe one more time?")
        );

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn no_failover_on_418() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let t1 = spawn_server(
            s1,
            ServerConfig {
                expect_settings: true,
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 418,
                    text: "I'm a teapot".to_string(),
                    body: "Would you like some tea instead?".to_string(),
                }),
                ..Default::default()
            },
        );
        let t2 = spawn_server(s2, ServerConfig::default());

        let mut sender = build_sender(vec![("127.0.0.1", p1), ("127.0.0.1", p2)]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        let flush_res = flush(&mut sender);
        assert!(flush_res.is_err());
        let flush_err = flush_res.err().unwrap();
        assert_eq!(
            flush_err.msg(),
            format!("Could not flush buffer: Would you like some tea instead?")
        );

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn failover_on_conn_err_3_endpoints() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);
        let s3 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let p3 = s3.port;
        let t1 = spawn_server(s1, ServerConfig::default());
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_settings: true,
                ..Default::default()
            },
        );

        let t3 = spawn_server(
            s3,
            ServerConfig {
                expect_write: WriteState::Accepting,
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![
            ("127.0.0.1", p1),
            ("127.0.0.1", p2),
            ("127.0.0.1", p3),
        ]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        flush(&mut sender).unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
    }

    #[test]
    fn all_fail() {
        let s1 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[1, 2], 127);
        let s2 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);
        let s3 = MockServer::new()
            .unwrap()
            .configure_settings_response(&[2], 127);

        let p1 = s1.port;
        let p2 = s2.port;
        let p3 = s3.port;
        let t1 = spawn_server(s1, ServerConfig::default());
        let t2 = spawn_server(
            s2,
            ServerConfig {
                expect_settings: true,
                ..Default::default()
            },
        );

        let t3 = spawn_server(
            s3,
            ServerConfig {
                expect_write: WriteState::Error(WriteStatusCode {
                    code: 404,
                    text: "Not Found".to_string(),
                    body: "".to_string(),
                }),
                ..Default::default()
            },
        );

        let mut sender = build_sender(vec![
            ("127.0.0.1", p1),
            ("127.0.0.1", p2),
            ("127.0.0.1", p3),
        ]);
        assert_eq!(sender.protocol_version(), ProtocolVersion::V2);
        let flush_res = flush(&mut sender);
        // don't want to give guarantees on what error is returned, so just checking that flush
        // returned an err
        assert!(flush_res.is_err());
        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
    }
}

#[cfg(feature = "sync-sender-http")]
#[test]
fn connect_timeout_uses_request_timeout() {
    use std::time::Instant;
    let request_timeout = Duration::from_millis(10);
    let builder = SenderBuilder::new(Protocol::Http, "127.0.0.2", "1111")
        .request_timeout(request_timeout)
        .unwrap()
        .protocol_version(ProtocolVersion::V2)
        .unwrap()
        .retry_timeout(Duration::from_millis(10))
        .unwrap()
        .request_min_throughput(0)
        .unwrap();
    let mut sender = builder.build().unwrap();
    let mut buf = sender.new_buffer();
    buf.table("x")
        .unwrap()
        .symbol("x", "x")
        .unwrap()
        .at_now()
        .unwrap();
    let start = Instant::now();
    sender
        .flush(&mut buf)
        .expect_err("Request did not time out");
    assert!(Instant::now() - start < Duration::from_secs(10));
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn auto_flush_off() {
    SenderBuilder::from_conf("tcps::addr=localhost;auto_flush=off;").unwrap();
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn auto_flush_unsupported() {
    assert_conf_err(
        SenderBuilder::from_conf("tcps::addr=localhost;auto_flush=on;"),
        "Invalid auto_flush value 'on'. This client does not support \
            auto-flush, so the only accepted value is 'off'",
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn auto_flush_rows_unsupported() {
    assert_conf_err(
        SenderBuilder::from_conf("tcps::addr=localhost;auto_flush_rows=100;"),
        "Invalid configuration parameter \"auto_flush_rows\". This client does not support auto-flush",
    );
}

#[cfg(feature = "sync-sender-tcp")]
#[test]
fn auto_flush_bytes_unsupported() {
    assert_conf_err(
        SenderBuilder::from_conf("tcps::addr=localhost;auto_flush_bytes=100;"),
        "Invalid configuration parameter \"auto_flush_bytes\". This client does not support auto-flush",
    );
}

fn assert_specified_eq<V: PartialEq + Debug, IntoV: Into<V>>(
    actual: &ConfigSetting<V>,
    expected: IntoV,
) {
    let expected = expected.into();
    if let ConfigSetting::Specified(actual_value) = actual {
        assert_eq!(actual_value, &expected);
    } else {
        panic!("Expected Specified({expected:?}), but got {actual:?}");
    }
}

fn assert_defaulted_eq<V: PartialEq + std::fmt::Debug, IntoV: Into<V>>(
    actual: &ConfigSetting<V>,
    expected: IntoV,
) {
    let expected = expected.into();
    if let ConfigSetting::Defaulted(actual_value) = actual {
        assert_eq!(actual_value, &expected);
    } else {
        panic!("Expected Defaulted({expected:?}), but got {actual:?}");
    }
}

fn assert_conf_err<T, M: AsRef<str>>(result: Result<T>, expect_msg: M) {
    let Err(err) = result else {
        panic!("Got Ok, expected ConfigError: {}", expect_msg.as_ref());
    };
    assert_eq!(err.code(), ErrorCode::ConfigError);
    assert_eq!(err.msg(), expect_msg.as_ref());
}

#[cfg(any(feature = "sync-sender-tcp", feature = "sync-sender-http"))]
fn assert_specified_endpoints_eq<V: Into<Vec<(&'static str, &'static str)>>>(
    actual: &ConfigSetting<Vec<Endpoint>>,
    expected: V,
) {
    let expected_pairs: Vec<(&str, &str)> = expected.into();
    if let ConfigSetting::Specified(actual_value) = actual {
        let pairs: Vec<(&str, &str)> = actual_value
            .iter()
            .map(|ep| (ep.host.as_str(), ep.port.as_str()))
            .collect();
        assert_eq!(pairs, expected_pairs);
    } else {
        panic!("Expected Specified(endpoints), but got {:?}", actual);
    }
}

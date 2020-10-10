use crate::models::{Book, Books, Config, Library};
use either::*;
use reqwest;

use serde_json::Value;
use std::io::Result;

pub(crate) fn access_api(config: Config) -> Result<String> {
    let params = [
        ("appkey", config.api_key),
        ("systemid", config.systemid),
        ("isbn", config.isbn.join(",")),
        ("callback", "no".to_string()),
    ];

    debug!("{}", &config.api_url);

    let client = reqwest::blocking::Client::new();

    let res_raw = match client.get(&config.api_url).form(&params).send() {
        Ok(res_raw) => res_raw,
        Err(e) => panic!(e),
    };

    match res_raw.json::<String>() {
        Ok(res) => Ok(res),
        Err(e) => {
            use std::io::{Error, ErrorKind};
            Err(Error::new(ErrorKind::Other, e))
        }
    }
}

fn parse_api_response(json: &str) -> Result<Either<String, Books>> {
    let json_obj: Value = serde_json::from_str(json).expect("failed to parse json");

    debug!("{}", json_obj);

    if json_obj["continue"].as_u64().unwrap() == 1u64 {
        return Ok(Left(json_obj["session"].as_str().unwrap().to_string()));
    } else {
        let mut books: Books = vec![];
        for book in json_obj["books"].as_object().unwrap() {
            let isbn = book.0.clone();

            let mut libraries: Vec<Library> = vec![];
            for sysid in book.1.as_object().unwrap() {
                debug!("systemid: {}", sysid.0);
                debug!("status: {}", sysid.1["status"]);
                debug!("reserveurl {}:", sysid.1["reserveurl"]);

                for lib in sysid.1["libkey"].as_object().unwrap() {
                    debug!("library: {:?}", lib);
                    let lib_name = lib.0.clone();
                    let lib_status = lib.1.as_str().unwrap().to_string();

                    libraries.push((lib_name, lib_status));
                }
            }

            books.push(Book::new(isbn, libraries));
        }

        return Ok(Right(books));
    }
}

#[cfg(test)]
mod tests {
    const json_res: &'static str = r#"{
        "session": "sessionid123", 
        "books": {
            "isbn1": {
                "Systemid_1": {
                    "status": "OK",
                    "reserveurl": "reserveurl1", 
                    "libkey": {
                        "図書館1": "貸出可",
                        "図書館2": "貸出中",
                        "図書館3": "館内のみ"
                    }
                }
            }, 
            "isbn2": {
                "Systemid_2": {
                    "status": "OK",
                    "reserveurl": "reserveurl2", 
                    "libkey": {
                    }
                }
            }
        },
        "continue": 0
      }"#;

    use super::*;

    #[test]
    fn parse_api_response_test() {
        env_logger::init();

        {
            debug!("returns session id if continue equals to 1");
            let json_polling_required = r#"
                {
                    "session": "xxxxxx",
                    "books": {},
                    "continue": 1
                }"#;

            match parse_api_response(json_polling_required) {
                Ok(either) => assert_eq!(either.expect_left("value is right"), "xxxxxx"),
                Err(e) => panic!(e),
            }
        }
        {
            debug!("returns struct books if continue equals to 0");
            match parse_api_response(json_res) {
                Ok(either) => {
                    let books = either.expect_right("value is left");
                    assert_eq!(books[0].isbn, "isbn1");

                    let lib_status = vec![
                        ("図書館1".to_string(), "貸出可".to_string()),
                        ("図書館2".to_string(), "貸出中".to_string()),
                        ("図書館3".to_string(), "館内のみ".to_string()),
                    ];
                    assert_eq!(books[0].libraries, lib_status);

                    assert_eq!(books[1].isbn, "isbn2");
                    assert!(books[1].libraries.is_empty());
                }
                Err(e) => panic!(e),
            }
        }
    }

    #[test]
    fn access_api_test() {
        env_logger::init();

        use httpmock::Method::GET;
        use httpmock::Mock;
        use httpmock::MockServer;

        let mock_server = MockServer::start();
        debug!("mock server is listening on {}", mock_server.address());

        let _api_mock = Mock::new()
            .expect_method(GET)
            .expect_path("/check")
            .return_status(200)
            .return_header("Content-Type", "application/json")
            .return_json_body(&json_res)
            .create_on(&mock_server);

        let config: Config = {
            let mut c: Config = Default::default();
            c.api_url = mock_server.url("/check");
            c
        };
        if let Ok(res) = access_api(config) {
            debug!("{}", res);
        };
    }
}

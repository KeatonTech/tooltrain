use lazy_static::lazy_static;
use commander::base::types::{Primitive, PrimitiveValue, StreamSpec};
use wasi::{
    http::{
        self,
        types::{Fields, IncomingBody, OutgoingRequest, Scheme},
    },
    io::streams::StreamError,
};

wit_bindgen::generate!({
    path: "../wit",
    world: "plugin",
    exports: {
        world: MastodonFeedProgram,
    },
});

mod parse;

struct MastodonFeedProgram;

impl Guest for MastodonFeedProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "Mastodon Public Feed".to_string(),
            description: "Returns the public timeline from a Mastodon instance".to_string(),
            arguments: vec![StreamSpec {
                name: "instance".to_string(),
                description: "The Mastodon instance to fetch the public feed from".to_string(),
                data_type: DataType::Primitive(Primitive::StringType),
            }],
        }
    }

    fn run(mut inputs: Vec<Value>) -> Result<String, String> {
        let Some(Value::PrimitiveValue(PrimitiveValue::StringValue(instance))) = inputs.pop()
        else {
            return Err("No instance name provided".to_string());
        };
        let mut headers = Fields::new();
        headers
            .set(
                &"User-Agent".to_string(),
                vec!["commander/0.1.0".as_bytes().to_vec()].as_slice(),
            )
            .unwrap();
        headers
            .set(
                &"Accept".to_string(),
                vec!["application/json".as_bytes().to_vec()].as_slice(),
            )
            .unwrap();
        let request = OutgoingRequest::new(Fields::new());
        request.set_authority(Some(&instance)).unwrap();
        request.set_scheme(Some(&Scheme::Https)).unwrap();
        request
            .set_path_with_query(Some("/api/v1/timelines/public"))
            .unwrap();
        let response_feed = http::outgoing_handler::handle(request, None)
            .map_err(|code| format!("Error constructing request: {:?}", code))?;
        response_feed.subscribe().block();
        let response = response_feed
            .get()
            .unwrap()
            .unwrap()
            .map_err(|e| format!("Error fetching public feed: {:?}", e))?;
        let incoming_body = response.consume().map_err(|_| "Empty body")?;
        let body = MastodonFeedProgram::read_incoming_body(incoming_body)?;
        let data: Vec<parse::Status> =
            serde_json::from_str(&body).map_err(|p| format!("Error parsing JSON: {:?}", p))?;

        add_output(
            "Feed",
            "The public feed from the Mastodon instance",
            &parse::OUTPUT_TABLE_TYPE,
            Some(&Value::TableValue(data.iter().map(parse::Status::as_output_value).collect())),
        );

        Ok("Done".to_string())
    }
}

impl MastodonFeedProgram {
    fn read_incoming_body(body: IncomingBody) -> Result<String, String> {
        let body_stream = body.stream().map_err(|_| "Error reading body")?;
        let mut body_bytes: Vec<u8> = vec![];
        loop {
            body_stream.subscribe().block();
            match body_stream.read(10240) {
                Ok(chunk) => {
                    body_bytes.extend_from_slice(&chunk);
                }
                Err(StreamError::Closed) => break,
                Err(e) => {
                    return Err(format!("Stream error while reading body: {:?}", e));
                }
            }
        }
        Ok(String::from_utf8_lossy(&body_bytes).to_string())
    }
}

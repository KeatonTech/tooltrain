use commander::base::{
    outputs::ListOutputRequest,
    types::{InputSpec, Primitive, PrimitiveValue},
};
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
});

mod parse;

struct MastodonFeedProgram;

impl Guest for MastodonFeedProgram {
    fn get_schema() -> Schema {
        Schema {
            name: "Mastodon Public Feed".to_string(),
            description: "Returns the public timeline from a Mastodon instance".to_string(),
            arguments: vec![InputSpec {
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

        let list_output = add_list_output(
            "Feed",
            "The public feed from the Mastodon instance",
            &parse::OUTPUT_TABLE_COLUMNS,
        );

        let first_page = MastodonFeedProgram::request_page(&instance, None)?;
        let first_page_values: Vec<Vec<PrimitiveValue>> =
            first_page.iter().map(|v| v.as_output_value()).collect();
        for value in first_page_values {
            list_output.add(&Value::CompoundValue(value));
        }
        list_output.set_has_more_rows(true);

        let mut prev_page = first_page;
        loop {
            match list_output.poll_request() {
                ListOutputRequest::Close => break,
                ListOutputRequest::LoadMore(_) => {
                    let max_id = prev_page.last().map(|s| s.id.clone());
                    let next_page = MastodonFeedProgram::request_page(&instance, max_id)?;
                    let next_page_values: Vec<Vec<PrimitiveValue>> =
                        next_page.iter().map(|v| v.as_output_value()).collect();
                    for value in next_page_values {
                        list_output.add(&Value::CompoundValue(value));
                    }
                    prev_page = next_page;
                }
            }
        }

        Ok("Done".to_string())
    }
}

impl MastodonFeedProgram {
    fn request_page(
        mastodon_instance: &str,
        newest_id: Option<String>,
    ) -> Result<Vec<parse::Status>, String> {
        let headers = Fields::new();
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
        request.set_authority(Some(mastodon_instance)).unwrap();
        request.set_scheme(Some(&Scheme::Https)).unwrap();
        let query_string = if let Some(id) = newest_id {
            format!("?max_id={}", id)
        } else {
            "".to_string()
        };
        let path = format!("/api/v1/timelines/public{}", query_string);
        request.set_path_with_query(Some(&path)).unwrap();
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
        serde_json::from_str(&body).map_err(|p| format!("Error parsing JSON: {:?}", p))
    }

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

export!(MastodonFeedProgram);

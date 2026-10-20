//! What a route says of itself in the OpenAPI document (`docs/api.md`, "The wire"): its operation id, tag,
//! summary, parameters, body, answers and codes, each written from the wire types' JSON Schemas. A route
//! declares its own beside its answer; the document is gathered from every route linked in.

use schemars::JsonSchema;
use schemars::generate::SchemaGenerator;
use serde_json::{Map, Value, json};

/// What a route takes as its body.
pub enum Body {
  Json(Value),
}

/// One route as the document states it.
pub struct Operation {
  pub id: &'static str,
  pub tag: &'static str,
  pub summary: &'static str,
  pub parameters: Vec<Value>,
  pub body: Option<Body>,
  /// The success answers, each by its status: the content by media type, or none for no body.
  pub answers: Vec<(u16, Option<Value>)>,
  /// Every code the route can answer.
  pub codes: &'static [&'static str],
}

impl Operation {
  /// The operation as the document writes it.
  pub fn value(self) -> Value {
    let mut responses = Map::new();

    for (status, content) in self.answers {
      let mut answer = json!({ "description": self.summary });

      if let Some(content) = content {
        answer["content"] = content;
      }

      responses.insert(status.to_string(), answer);
    }

    responses.insert("default".into(), json!({ "$ref": "#/components/responses/Error" }));

    let mut operation = json!({
      "operationId": self.id,
      "tags": [self.tag],
      "summary": self.summary,
      "responses": responses,
      "x-codes": self.codes,
    });

    if !self.parameters.is_empty() {
      operation["parameters"] = self.parameters.into();
    }

    if let Some(Body::Json(schema)) = self.body {
      operation["requestBody"] =
        json!({ "required": true, "content": { "application/json": { "schema": schema } } });
    }

    operation
  }
}

/// A JSON answer of this type.
pub fn json_answer<Answer: JsonSchema>(generator: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({ "application/json": { "schema": generator.subschema_for::<Answer>() } }))
}

/// An answer of bytes.
pub fn bytes_answer() -> Option<Value> {
  Some(
    json!({ "application/octet-stream": { "schema": { "type": "string", "contentEncoding": "binary" } } }),
  )
}

/// A JSON body of this type.
pub fn json_body<Sent: JsonSchema>(generator: &mut SchemaGenerator) -> Option<Body> {
  Some(Body::Json(generator.subschema_for::<Sent>().to_value()))
}

/// A path parameter that is a member's id.
pub fn id_parameter(name: &str) -> Value {
  json!({ "name": name, "in": "path", "required": true, "schema": { "type": "integer", "format": "int64" } })
}

/// A path parameter that is a string.
pub fn text_parameter(name: &str, description: &str) -> Value {
  json!({ "name": name, "in": "path", "required": true, "description": description, "schema": { "type": "string" } })
}

/// A header parameter.
pub fn header(name: &str, description: &str) -> Value {
  json!({ "name": name, "in": "header", "required": false, "description": description, "schema": { "type": "string" } })
}

/// The query parameters of a query type, each property one parameter.
pub fn query<Query: JsonSchema>(generator: &mut SchemaGenerator) -> Vec<Value> {
  let schema = Query::json_schema(generator).to_value();
  let Some(properties) = schema["properties"].as_object() else {
    return Vec::new();
  };

  properties
    .iter()
    .map(|(name, property)| {
      let mut parameter = json!({ "name": name, "in": "query", "required": false, "schema": property });

      if let Some(description) = property.get("description") {
        parameter["description"] = description.clone();
      }

      parameter
    })
    .collect()
}

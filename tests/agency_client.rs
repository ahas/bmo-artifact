//! The bmo-agency client's call shapes (`docs/api.md`, "bmo-agency"), against the stand-in in
//! `tests/support/agency.rs`: an unset address, a read of one thing that answers 404, a refusal's status,
//! code and sentence handed on, a call that never reaches bmo-agency, and the paths and queries it asks.

mod support;

use bmo_artifact::clients::agency::{Agency, AgencyClient, AgencyFault, PlaceWrite, RouteAsked};
use support::agency::{Book, standing_in};

#[tokio::test]
async fn every_call_is_refused_where_the_agencys_address_is_not_set() {
  let agency = AgencyClient::new(None);
  let said = "the agency's address is not set: set agency under [units] in the settings";

  assert_eq!(agency.providers().await, Err(AgencyFault::Failed(said.to_owned())));
  assert_eq!(agency.stop_apart("apart-1").await, Err(AgencyFault::Failed(said.to_owned())));
}

#[tokio::test]
async fn a_read_of_one_thing_that_answers_404_is_nothing_found() {
  let (_, agency) = standing_in(Book::lending()).await;
  let asked = RouteAsked {
    work_key: None,
    provider: Some("prov-a".to_owned()),
    tier: "no-such-tier".to_owned(),
  };

  assert_eq!(agency.registration_by_home("/homes/nobody").await, Ok(None));
  assert_eq!(agency.request("req-404").await, Ok(None));
  assert_eq!(agency.resolve(&asked).await, Ok(None));
}

#[tokio::test]
async fn a_refusal_is_handed_on_with_its_status_its_code_and_its_sentence() {
  let book = Book {
    refused_efforts: vec!["max".to_owned()],
    limit_refusal: Some((
      422,
      "limit_refused".to_owned(),
      "prov-a holds at most 2 small places".to_owned(),
    )),
    ..Book::lending()
  };
  let (_, agency) = standing_in(book).await;

  assert_eq!(
    agency.check_effort("prov-a", "model-b", "max").await,
    Err(AgencyFault::Refused {
      status: 422,
      code: "effort_refused".to_owned(),
      error: "model-b does not take the max effort".to_owned(),
    })
  );
  assert_eq!(agency.check_effort("prov-a", "model-b", "high").await, Ok(()));
  assert_eq!(
    agency.set_limit("prov-a", "small", 9).await,
    Err(AgencyFault::Refused {
      status: 422,
      code: "limit_refused".to_owned(),
      error: "prov-a holds at most 2 small places".to_owned(),
    })
  );
  // A read of a thing that must exist hands its 404 on as the refusal it is.
  assert!(matches!(agency.lease("req-404").await, Err(AgencyFault::Refused { status: 404, .. })));
}

#[tokio::test]
async fn a_call_that_never_reaches_bmo_agency_is_a_failure_naming_the_call() {
  let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}", closed.local_addr().unwrap());

  drop(closed);

  let fault = AgencyClient::new(Some(address)).tiers().await.unwrap_err();

  assert!(
    matches!(&fault, AgencyFault::Failed(said) if said.starts_with("bmo-agency could not list the tiers: ")),
    "{fault}"
  );
}

#[tokio::test]
async fn each_call_goes_to_its_path_with_its_query_under_an_address_with_a_trailing_slash() {
  let (stand, agency) = standing_in(Book::lending()).await;
  let write = PlaceWrite {
    provider: "prov-a".to_owned(),
    model: "model-a".to_owned(),
    effort: None,
    upstream: None,
  };

  agency.reservations(Some("search-run:9")).await.unwrap();
  agency.reservations(None).await.unwrap();
  agency.write_place("branch-line:prov-a:model-a:", &write).await.unwrap();
  agency.registration_by_home("/homes/a b").await.unwrap();
  agency.reset_limits().await.unwrap();

  assert_eq!(
    stand.book().calls,
    [
      "GET /pool/reservations?owner=search-run%3A9",
      "GET /pool/reservations",
      "PUT /pool/places/branch-line:prov-a:model-a:/write",
      "GET /registrations/by-home?home=%2Fhomes%2Fa+b",
      "DELETE /pool/limits",
    ]
  );
  assert_eq!(stand.book().writes, [("branch-line:prov-a:model-a:".to_owned(), write)]);
}

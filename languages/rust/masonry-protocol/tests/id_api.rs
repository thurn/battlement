use std::any::TypeId;

use masonry_protocol::{IdError, ObjectId, SessionId};
use schemars::JsonSchema;

#[test]
fn role_aliases_expose_the_shared_id_api_to_consumers() {
    let session = SessionId::new_v4();
    let object = ObjectId::from_uuid(session.into_uuid()).unwrap();

    assert_ne!(TypeId::of::<SessionId>(), TypeId::of::<ObjectId>());
    assert_eq!(SessionId::schema_name(), "SessionId");
    assert_eq!(ObjectId::schema_name(), "ObjectId");
    assert_eq!(session.to_string(), object.to_string());
    assert_eq!(SessionId::from_uuid(uuid::Uuid::nil()), Err(IdError::Nil));
}

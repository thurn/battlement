use std::collections::HashSet;
use std::mem;

use masonry::{
    ClientMessage, Command, Connect, CustomCommand, Response, ResponseMessage, ScreenSize,
    messagepack,
};
use serde::{Deserialize, Serialize};

const CONNECT: &[u8] =
    include_bytes!("../../../Packages/com.masonry.client/Tests/Fixtures/csharp-connect.msgpack");
const RESPONSE: &[u8] =
    include_bytes!("../../../Packages/com.masonry.client/Tests/Fixtures/csharp-response.msgpack");
const CUSTOM_RESPONSE: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-custom-response.msgpack"
);
const RUST_RESPONSE: &[u8] =
    include_bytes!("../../../Packages/com.masonry.client/Tests/Fixtures/rust-response.msgpack");
const RUST_CUSTOM_RESPONSE: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/rust-custom-response.msgpack"
);
const ACTIONS: [&[u8]; 9] = [
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-enter.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-exit.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-down.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-up.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-pointer-click.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-drag-start.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-drag-end.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-key-down.msgpack"
    ),
    include_bytes!(
        "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-key-up.msgpack"
    ),
];
const CUSTOM_ACTION: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-custom.msgpack"
);
const BATCH_FAILED: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-batch-failed.msgpack"
);
const OPERATION_FAILED: &[u8] = include_bytes!(
    "../../../Packages/com.masonry.client/Tests/Fixtures/csharp-client-operation-failed.msgpack"
);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SamplePayload {
    name: String,
    count: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum SampleError {
    IllegalMove,
    NotReady,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
enum GameCommand {
    Core(Command),
    Custom(CustomCommand<SamplePayload>),
}

#[test]
fn csharp_connect_decodes_and_reproduces_exactly() {
    let connect: Connect = messagepack::from_slice(CONNECT).expect("C# connect must decode");

    assert_eq!(connect.platform, "macOS");
    assert_eq!(
        connect.screen,
        ScreenSize {
            width: 2560,
            height: 1440
        }
    );
    assert_eq!(
        connect.custom_command_types,
        ["cards.draw", "cards.shuffle"]
    );
    assert_eq!(
        messagepack::to_vec(&connect).expect("connect must encode"),
        CONNECT
    );
}

#[test]
fn csharp_comprehensive_response_covers_every_core_command() {
    let response: Response = messagepack::from_slice(RESPONSE).expect("C# response must decode");
    let ResponseMessage::Batch(batch) = &response.messages[1] else {
        panic!("second response message must be a batch");
    };
    let commands = &batch.groups[0].commands;
    let variants: HashSet<_> = commands
        .iter()
        .map(|command| mem::discriminant(&command.body))
        .collect();

    assert!(matches!(response.messages[0], ResponseMessage::Snapshot(_)));
    assert_eq!(commands.len(), 74);
    assert_eq!(variants.len(), 73);
    assert_eq!(
        messagepack::to_vec(&response).expect("response must encode"),
        RESPONSE
    );
}

#[test]
fn csharp_custom_response_decodes_with_game_owned_payload() {
    let response: Response<GameCommand> =
        messagepack::from_slice(CUSTOM_RESPONSE).expect("custom response must decode");
    let ResponseMessage::Batch(batch) = &response.messages[0] else {
        panic!("response must contain a batch");
    };

    assert!(matches!(
        &batch.groups[0].commands[0],
        GameCommand::Custom(CustomCommand { command_type, payload, .. })
            if command_type == "cards.reveal" && payload.count == 2
    ));
    assert_eq!(
        messagepack::to_vec(&response).expect("custom response must encode"),
        CUSTOM_RESPONSE
    );
}

#[test]
fn rust_corpus_is_reproduced_by_the_rust_encoder() {
    let response: Response =
        messagepack::from_slice(RUST_RESPONSE).expect("Rust response must decode");
    let custom: Response<GameCommand> =
        messagepack::from_slice(RUST_CUSTOM_RESPONSE).expect("Rust custom response must decode");

    assert_eq!(messagepack::to_vec(&response).unwrap(), RUST_RESPONSE);
    assert_eq!(messagepack::to_vec(&custom).unwrap(), RUST_CUSTOM_RESPONSE);
}

#[test]
fn csharp_client_message_union_decodes_and_reproduces_exactly() {
    let actions: Vec<ClientMessage<SamplePayload, SampleError>> = ACTIONS
        .iter()
        .map(|bytes| messagepack::from_slice(bytes).expect("action must decode"))
        .collect();
    let custom: ClientMessage<SamplePayload, SampleError> =
        messagepack::from_slice(CUSTOM_ACTION).expect("custom action must decode");
    let batch_failed: ClientMessage<SamplePayload, SampleError> =
        messagepack::from_slice(BATCH_FAILED).expect("batch failure must decode");
    let operation_failed: ClientMessage<SamplePayload, SampleError> =
        messagepack::from_slice(OPERATION_FAILED).expect("operation failure must decode");

    let action_variants: HashSet<_> = actions
        .iter()
        .map(|message| match message {
            ClientMessage::Action(action) => mem::discriminant(&action.body),
            _ => panic!("expected an action"),
        })
        .collect();
    assert_eq!(action_variants.len(), 9);
    assert!(matches!(custom, ClientMessage::CustomAction(_)));
    assert!(matches!(batch_failed, ClientMessage::BatchFailed(_)));
    assert!(matches!(
        operation_failed,
        ClientMessage::OperationFailed(_)
    ));
    for (action, bytes) in actions.iter().zip(ACTIONS) {
        assert_eq!(messagepack::to_vec(action).unwrap(), bytes);
    }
    assert_eq!(messagepack::to_vec(&custom).unwrap(), CUSTOM_ACTION);
    assert_eq!(messagepack::to_vec(&batch_failed).unwrap(), BATCH_FAILED);
    assert_eq!(
        messagepack::to_vec(&operation_failed).unwrap(),
        OPERATION_FAILED
    );
}

#[test]
fn rust_round_trip_and_malformed_inputs_are_rejected() {
    let connect = Connect::new(
        "Linux",
        "6000.5.8f1",
        ScreenSize {
            width: 1920,
            height: 1080,
        },
    );
    let bytes = messagepack::to_vec(&connect).expect("connect must encode");
    assert_eq!(messagepack::from_slice::<Connect>(&bytes).unwrap(), connect);

    let mut trailing = bytes.clone();
    trailing.push(0xc0);
    assert!(messagepack::from_slice::<Connect>(&trailing).is_err());
    assert!(messagepack::from_slice::<Connect>(&bytes[..bytes.len() - 1]).is_err());
    assert!(messagepack::from_slice::<Connect>(&[0x95]).is_err());
    assert!(
        messagepack::from_slice::<ClientMessage<(), ()>>(&[
            0x81, 0xa7, b'U', b'n', b'k', b'n', b'o', b'w', b'n', 0xc0
        ])
        .is_err()
    );
    assert!(
        messagepack::from_slice::<Connect>(&[
            0x96, 0xa1, b'x', 0xa1, b'y', 0x92, 0xcf, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0x90, 0xc0, 0xc0
        ])
        .is_err()
    );
}

# JSON protocol migration design

Status: proposed implementation contract

## Summary

Masonry will replace its MessagePack wire encoding with minified UTF-8 JSON.
The Rust protocol types remain the authoritative model and keep their existing
public shape, builders, validation, and Serde derives. Rust will serialize those
types through `serde_json`; it will not acquire JSON-only DTOs, field attributes,
or conversion layers.

Unity will continue to expose strongly typed C# records. Newtonsoft.Json will
handle ordinary records automatically, while a small Masonry-specific layer
will cover the places where the ergonomic C# model deliberately differs from
Serde's natural JSON representation. In particular, reusable converters will
handle externally tagged unions, typed scalar wrappers, millisecond durations,
and flattened property commands. The new codec must not reproduce the current
field-by-field reader and writer implementation.

This is an atomic encoding change. The completed implementation supports JSON
only; it does not negotiate, detect, or retain MessagePack.

## Motivation

The current Unity protocol and MessagePack directories contain about 6,800
lines. Roughly 4,200 of those lines implement the MessagePack boundary. A core
command is normally repeated as a C# record, a writer branch, and a reader
branch. Adding approximately one hundred more protocol records would make that
repetition a significant source of schema drift and review burden.

Changing to Protobuf would not solve that problem under Masonry's constraints.
The existing Rust structs would require conversions to generated Protobuf
messages, and retaining the C# records would add a second conversion layer.
JSON can serialize the unchanged Serde model directly and can populate the C#
records without per-field codec code.

## Goals

- Keep every existing Rust protocol struct, enum, builder, and validation API
  unchanged.
- Keep the C# record hierarchy and ergonomic property names by default.
- Make an ordinary new record require no JSON codec code.
- Make a new union case require one declarative registration, not separate read
  and write implementations.
- Automatically serialize normal game-owned payload and error types, with an
  explicit override for exceptional types.
- Preserve the existing transport, host, scheduling, and native buffer
  boundaries.
- Retain focused cross-language and malformed-input coverage without building
  a schema system or a second validation framework.

## Non-goals

- Preserving MessagePack compatibility or supporting a mixed deployment.
- Publishing a language-neutral schema or supporting arbitrary client
  languages.
- Generating Rust or C# domain types.
- Changing the native ABI, HTTP endpoint structure, or host execution model.
- Moving Rust validation rules into JSON metadata or Unity converters.
- Making JSON property order part of the protocol contract.

## Wire representation

The JSON representation is exactly the natural representation produced by
`serde_json` for the existing Rust types:

- Structs are objects whose property names are the Rust field names, such as
  `session_id` and `custom_command_types`.
- Unit enum variants are strings.
- Variants containing data use Serde's externally tagged single-property
  object, such as `{ "SceneLoad": { ... } }`.
- `Option<T>` is either `null` or the contained value. Missing optional object
  properties are accepted according to normal Serde behavior.
- UUIDs are lowercase hyphenated strings. Masonry's typed IDs continue to
  reject the all-zero UUID.
- Asset addresses and other string-backed wrappers are JSON strings.
- Durations remain nonnegative integer milliseconds in fields ending in
  `_ms`.
- Enums retain their Rust variant spelling. They are not converted to camel
  case or numeric values.
- Every non-optional struct field is emitted, including fields whose value is a
  Rust or C# constructor default.
- Maps become JSON objects. The protocol continues to use string keys for all
  maps that cross this boundary.

Writers produce minified UTF-8 without a byte-order mark. Readers accept
insignificant JSON whitespace. Object property order is irrelevant, so tests
compare decoded values or parsed JSON rather than serialized bytes.

## Rust boundary

The `masonry` crate will depend on the workspace's pinned `serde_json` and stop
depending on `rmp-serde`. Its encoding-specific public module becomes
`masonry::json`, retaining the familiar operations:

- `to_vec<T>(&T) -> Result<Vec<u8>, serde_json::Error>`
- `from_slice<T>(&[u8]) -> Result<T, serde_json::Error>`

`from_slice` must consume exactly one JSON value and reject trailing content.
Serde's recursion limit remains enabled. Protocol values are validated through
the same existing `Validate` paths as today; JSON decoding does not duplicate
cross-field validation.

This codec-module replacement is the only intended Rust public API change. The
engine trait, all protocol types, their fields and variants, construction APIs,
and generic custom payload types remain unchanged.

`masonry-native` will call `masonry::json` at the same points where it currently
calls `masonry::messagepack`. The C ABI continues to borrow input bytes and
return native-owned output buffers with the same status values and lifetime
rules. Only the bytes and encoding-related diagnostics change.

## Unity JSON boundary

### Package and assembly

The Unity package will depend on `com.unity.nuget.newtonsoft-json` 3.2.2, the
package bundled with Unity 6000.5 and synchronized to Newtonsoft.Json 13.0.2.
The bundled MessagePack assemblies and their license files will be removed.

`Masonry.MessagePack` and `MasonryMessagePack` will be replaced by
`Masonry.Json` and `MasonryJson`. `MasonryJson.Instance` implements the existing
`IMasonryProtocolCodec` boundary and becomes the production codec selected by
`MasonryBootstrap`. The base codec interface remains encoding-neutral and
continues to exchange `byte[]` and `ReadOnlyMemory<byte>`.

### Serializer configuration

Masonry owns one immutable base configuration. It uses:

- snake-case property names;
- string enum values with their declared C# spelling;
- included null and default values;
- invariant numeric formatting;
- no type-name metadata;
- a maximum depth of 128;
- strict UTF-8 decoding;
- additional-content checks after the root value;
- the union and scalar converters described below.

Unknown externally tagged variants are errors. Unknown properties on a known
record are ignored, matching default Serde behavior. Non-optional constructor
properties are required; absent nullable properties represent Rust `None`.
A contract resolver will apply those rules centrally instead of placing
required-property attributes on every record.

Records with multiple public constructors will identify their canonical
positional constructor for JSON when Newtonsoft cannot select it
unambiguously. This is metadata only and does not change construction or record
semantics.

### Record mapping

The existing public records remain the preferred C# API. JSON mapping follows
these rules:

1. Matching properties use the global snake-case convention automatically.
2. A genuinely different wire name, such as `Command.Id` versus
   `command_id`, receives a local `JsonProperty` name.
3. A record is reshaped only when preserving it would require substantial
   dedicated conversion logic. Saving a few mapping lines does not justify
   making a common command awkward to construct or pattern-match.
4. New records should use clear C# names. They need not copy awkward Rust
   payload-wrapper names merely to avoid one property annotation.

Records remain immutable positional records and retain value equality,
deconstruction, pattern matching, optional constructor defaults, and `with`
expressions.

### Typed scalar converters

Small reusable converters will encode all protocol ID wrappers as UUID strings
and all address wrappers as strings. They validate nullability and the nonzero
UUID invariant when constructing the existing readonly structs.

TimeSpan properties correspond to Rust integer fields ending in `_ms`. The
contract resolver maps the property name to that suffix, and a converter
requires a nonnegative whole-millisecond integer. It rejects fractional,
negative, and overflowing values. Existing protocol validation continues to
enforce field-specific upper bounds.

### Externally tagged unions

A shared externally tagged union converter will support four payload shapes:

- a unit variant represented by a JSON string;
- a newtype variant represented by one tag and one scalar or nested value;
- a record variant represented by one tag and one object;
- a property-command variant represented by `on_conflict` plus a nested
  `payload` object.

Each abstract union base owns an explicit static registration table mapping a
Serde variant tag to its concrete C# record and payload shape. One registration
supports both reading and writing. The implementation will not scan assemblies,
emit generated code, or maintain separate read and write switches.

Property-command records remain flat in C#. When writing, the adapter moves
`OnConflict` into `on_conflict` and nests the remaining record properties under
`payload`. When reading, it performs the inverse merge before asking
Newtonsoft.Json to construct the concrete record. This single adapter covers
all current and future `IPropertyCommandBody` records.

Variants with exceptional Rust/C# shapes may supply a small case-specific
payload adapter in their registration. Such adapters are the exception; the
design must not regress into one custom converter per variant.

## Game-owned extensions

Normal custom command payloads, custom action payloads, and error-code enums use
the same snake-case and string-enum conventions automatically. Game code no
longer has to implement a formatter for ordinary records or enums.

The public registration and action APIs retain their existing generic type
parameters but replace required MessagePack formatters with optional typed
Newtonsoft.Json converters:

- `RegisterCommand<TPayload, TError>` accepts optional payload and error
  converters after the handler.
- `EmitCustomAction<TPayload>` accepts an optional payload converter.
- The extension codec's custom action and failure operations accept the same
  optional typed converters.

When an override is present, it applies only to that game-owned payload or
error value. It must not modify the shared core serializer configuration.
Registration continues to reject duplicate or invalid command namespaces.
Payload conversion failures continue to produce an invalid custom command and
the existing `InvalidEncoding` host behavior.

## Transports

Transport control flow and payload limits do not change. Both native and HTTP
transports carry the same JSON document for a given protocol value.

The HTTP transport will send and accept `application/json`; POST bodies are
UTF-8 JSON. Endpoints, timeouts, synchronous behavior, response-size limits,
204 polling behavior, and error handling remain unchanged.

The native transport continues to treat successful buffers as opaque bytes.
Native function names and structs remain unchanged because none of them expose
the encoding in their ABI. Parameter names, comments, logs, and diagnostics
will use `json` or encoding-neutral `payload` terminology rather than
`messagePack`.

## Failure and safety behavior

The JSON codec must reject:

- malformed or truncated UTF-8 JSON;
- trailing content after the root value;
- an unknown union variant or an invalid union payload shape;
- a missing non-optional field;
- a null required string, collection, record, or scalar wrapper;
- an invalid or all-zero UUID;
- an invalid enum string;
- an integer outside the destination type's range;
- a duration that is negative, fractional, or not representable as a
  `TimeSpan` in whole milliseconds;
- nesting deeper than 128 levels.

JSON decoding is not a replacement for `Validate`. Finite-number rules,
collection limits, uniqueness, references, hierarchy, and other semantic
invariants remain in their current validation owners. The transport-level
16 MiB response limit remains the first bound on untrusted input.

## Migration sequence

The implementation will be developed on one branch and land atomically:

1. Add the Rust JSON codec and convert Rust/native fixtures and diagnostics
   while leaving every protocol type untouched.
2. Add the Newtonsoft.Json dependency, base configuration, scalar converters,
   and reusable union converter.
3. Apply only the C# record metadata and exceptional mappings required by the
   Rust-produced comprehensive fixture.
4. Replace the public custom formatter parameters with automatic JSON plus
   optional converter overrides, then migrate custom command tests and samples.
5. Select `MasonryJson` in the host, update native and HTTP fixtures, and change
   HTTP media types.
6. Delete the MessagePack assembly, bundled dependencies, handwritten codec,
   formatter fixtures, and binary fixture corpus.
7. Update the normative technical design, fake-client description, API
   comments, logs, and package assembly checks to name JSON.

The final codebase must contain no encoding negotiation or dormant MessagePack
path. Intermediate commits need only compile when useful during development;
the promoted task is the complete atomic migration.

## Test strategy

The existing host suite remains the primary behavioral coverage. Tests that
currently use `MasonryMessagePack` as fixture plumbing will use `MasonryJson`
without duplicating their scheduling, world, operation, or error assertions.

Focused codec coverage consists of:

1. A Rust-produced comprehensive response that C# decodes into every concrete
   built-in command type, then reserializes to structurally equivalent JSON.
2. C#-produced connect, built-in action, custom action, batch-failure, and
   operation-failure documents that Rust decodes into the expected values.
3. One game-owned custom command payload and one custom error enum exercising
   automatic conventions, plus one converter override proving the escape hatch.
4. A compact malformed corpus covering trailing JSON, truncation, unknown
   variants, missing required fields, nil UUIDs, overflow, invalid duration,
   and excessive nesting.
5. Existing native and HTTP release scenarios proving both transports exchange
   JSON with the unchanged lifecycle and ordering behavior.

Fixture assertions compare protocol values or parsed JSON trees, never raw JSON
bytes or object property order. The comprehensive response remains the
exhaustiveness guard: adding a Rust command without its C# record or union
registration must fail interop tests.

The normal CI suite remains sufficient: Rust tests, Unity compilation and Edit
Mode tests, native and HTTP fixtures, required IL2CPP builds, package assembly
checks, and the existing performance smoke tests. The migration adds no new
schema validator, generated-code check, fuzzing system, or standalone benchmark
suite. If an existing performance gate exposes a JSON regression, that concrete
failure will determine any optimization work.

## Acceptance criteria

- The public Rust protocol types and builders are unchanged.
- Rust and C# exchange all built-in messages and representative game-owned
  values as JSON through native and HTTP transports.
- The ergonomic C# record hierarchy remains available to host and game code.
- Ordinary records use automatic serialization; there are no per-field read and
  write methods.
- Every union case has one authoritative registration.
- Ordinary custom payloads and error enums require no formatter code.
- MessagePack dependencies, assemblies, codec sources, fixtures, MIME types,
  comments, and public formatter parameters are removed.
- The JSON-specific runtime implementation is materially smaller than the
  deleted MessagePack-specific implementation.
- The complete existing CI entry point passes.

## Alternatives rejected

### Protobuf

Keeping the existing Rust structs would require parallel generated Rust
messages and conversion code. Keeping the C# records would require another
conversion layer. Protobuf therefore increases the number of maintained
representations instead of reducing it.

### System.Text.Json

System.Text.Json can represent the data, but Newtonsoft.Json has the more
mature Unity package and token/converter APIs needed for Serde's externally
tagged unions. Selecting it avoids a separate Unity/AOT integration project.

### JSON Schema or generated DTOs

Masonry previously evaluated schema-driven C# projection. General-purpose
generators produced poor union shapes and coupled schema concerns to the Rust
model. The proposed reusable converters preserve deliberate C# ergonomics
without introducing an intermediate schema or generated source.

### Dual JSON and MessagePack support

Two codecs would retain the code being removed and double the interop matrix.
Masonry deploys its Rust engine and Unity package together and does not require
encoding negotiation or backward compatibility.

# QueryGraph Rust Engineering Guide

This is the normative Rust coding guide for human contributors and coding
agents working on QueryGraph. Read it before changing Rust source, tests,
benchmarks, Cargo configuration, or a Rust-facing wire contract.

The objective is not merely code that compiles. QueryGraph Rust code must be
correct, secure, reusable, modular, readable, DRY, and fast. It should use
Rust's functional and type-system strengths to make invalid states difficult or
impossible to express, while stopping before an abstraction becomes type
gymnastics. Performance work must preserve those qualities and be justified by
measurement.

This document governs the QueryGraph crate rooted at this repository's
`Cargo.toml`. A nested checkout, including `sail/`, is an independent repository:
follow its nearest guidance and do not modify it unless the task explicitly
includes it. Where this guide and a task-specific requirement differ, the
task-specific requirement wins. `MUST`, `SHOULD`, and `MAY` have their usual
normative meanings.

Last research review: 2026-08-22.

## Contents

1. [The standard](#the-standard)
2. [QueryGraph architecture and dependency direction](#querygraph-architecture-and-dependency-direction)
3. [Required workflow for coding agents](#required-workflow-for-coding-agents)
4. [Model the domain with algebraic data types](#model-the-domain-with-algebraic-data-types)
5. [Functional core, imperative shell](#functional-core-imperative-shell)
6. [Type-driven design without type gymnastics](#type-driven-design-without-type-gymnastics)
7. [DRYness and abstraction design](#dryness-and-abstraction-design)
8. [Modules and public APIs](#modules-and-public-apis)
9. [Ownership, borrowing, and resources](#ownership-borrowing-and-resources)
10. [Errors and failure semantics](#errors-and-failure-semantics)
11. [Async and concurrency](#async-and-concurrency)
12. [Performance engineering](#performance-engineering)
13. [Unsafe Rust and FFI](#unsafe-rust-and-ffi)
14. [Serialization, persistence, and security boundaries](#serialization-persistence-and-security-boundaries)
15. [Tests live in separate files](#tests-live-in-separate-files)
16. [Documentation, comments, and naming](#documentation-comments-and-naming)
17. [Dependencies, features, and toolchains](#dependencies-features-and-toolchains)
18. [Patterns to reject](#patterns-to-reject)
19. [Canonical QueryGraph patterns](#canonical-querygraph-patterns)
20. [Review and completion checklists](#review-and-completion-checklists)
21. [Intellectual lineage and primary sources](#intellectual-lineage-and-primary-sources)

## The standard

Use this priority order when goals pull in different directions:

1. Soundness, correctness, security, and durable data integrity.
2. Explicit domain semantics and enforced invariants.
3. Readability, local reasoning, and maintainable boundaries.
4. Measured runtime performance and bounded resource use.
5. Compile time, binary size, and operational simplicity.
6. Brevity and syntactic cleverness.

A lower item never silently defeats a higher one. A performance exception may
introduce carefully isolated duplication, mutation, dynamic dispatch, or lower
level code only when a representative benchmark demonstrates a meaningful win.
The exception must include the evidence, the preserved invariant, and a test
that keeps equivalent implementations behaviorally aligned.

### The governing design rules

- Represent domain alternatives with `enum`, not flag matrices, magic strings,
  null-like sentinels, or loosely related `Option` fields.
- Represent domain conjunctions with cohesive `struct` product types.
- Parse untrusted or weakly typed input once at a boundary; pass validated,
  semantically named types through the core.
- Keep the decision-making core pure where practical. Put I/O, clocks, random
  values, environment access, databases, network calls, and task spawning at
  explicit edges.
- Prefer immutable values and consuming transitions. Keep required mutation
  narrow, owned, and visibly scoped.
- Eliminate semantic duplication. Do not invent an abstraction merely because
  two snippets look alike.
- Use the least powerful abstraction that expresses the invariant cleanly:
  concrete function, newtype, enum/struct, generic function, trait, typestate,
  macro—in roughly that order.
- Advanced types must improve the caller's code, compiler diagnostics, or
  guarantees enough to pay for their cognitive and compilation costs.
- Optimize algorithms and data movement before syntax. Profile release code,
  benchmark representative work, and retain before/after evidence.
- Keep every test body outside production source files.
- Keep safe Rust the default. `unsafe` is exceptional infrastructure requiring
  an explicit soundness argument and specialized verification.

### Productive reliability

Niko Matsakis describes Rust's design in terms of accessibility, early error
detection, transparent meaning, reliability, and efficiency. QueryGraph adopts
the same balance. A theoretically stronger API that only one specialist can
understand is usually weaker engineering than a slightly less ambitious API
that every maintainer can safely evolve. Conversely, leaving an authorization,
verification, identity, or persistence invariant in prose when a small type can
enforce it is not simplicity—it is deferred complexity.

Ask of every design:

- What invalid values or transitions can this representation express?
- Where is each invariant established, and can it be established only once?
- What can a tired maintainer or context-limited coding agent accidentally do?
- Does the type signature reveal the important effects and failure modes?
- Can the implementation be replaced without leaking adapter details?
- Is the runtime and compile-time cost known, or merely assumed?

## QueryGraph architecture and dependency direction

QueryGraph integrates governed semantic models, identities and policy,
catalog/graph systems, cognition, lakehouse execution, lineage, agents, and
transport protocols. Preserve a dependency direction that lets the domain be
understood and tested without booting the world:

```text
transport / CLI
      |
application orchestration
      |
domain types + pure policy / planning
      ^
adapter implementations -> external systems and wire formats
```

The arrow from an adapter points inward because the adapter implements a
capability needed by the application; the domain must not know an HTTP client,
database driver, or vendor SDK merely because one implementation uses it.

### Current code map

Use this map to decide where a change belongs:

- Domain representations: `croissant`, `cdif`, `codata`, `did`, `odrl`, `osi`,
  `rbac`, `lakehouse::types`, `qglake::model`, and focused model modules.
- Application composition and decisions: `navigator`, `cognition`, `qglake`,
  `validation`, and `stack`.
- External adapters: `lakecat`, `sail`, `dataverse`, `lineage`, `agent`, and
  memory integrations.
- Transports and entry points: `server`, `mcp`, `a2a`, and `main`.
- Graph interpretation: `cypher` and graph-facing adapter code.

This is a direction, not permission to create ceremonial layers. A tiny pure
module does not need a trait, repository interface, service object, and factory.
Introduce a boundary when there is a real effect, policy seam, substitutable
implementation, test seam, or ownership reason.

### Boundary rules

- Domain modules MUST NOT depend on Axum extractors, request/response DTOs,
  Reqwest clients, filesystem paths chosen by a CLI, or database session types.
- Transport modules translate typed application outcomes into protocol status,
  headers, and JSON. They do not own policy decisions.
- Adapter modules translate external errors and representations at one narrow
  boundary. External types must not diffuse through the core.
- `serde_json::Value` is acceptable for genuinely dynamic standards and at wire
  boundaries. It SHOULD NOT be the internal representation of known fields or
  finite alternatives.
- Cross-layer conversion should be explicit (`TryFrom`, `From`, a named parser,
  or a named projection), testable, and located with the destination type or
  boundary adapter.
- Security-sensitive values—verified identities, grants, proofs, policy
  digests, signed envelopes, authorized plans—must have types that distinguish
  them from unverified input.
- Keep public exports curated. A module hierarchy is not automatically an API.
  Default to private, then `pub(super)`, then `pub(crate)`, and use `pub` only
  for an intentional consumer contract.

### Physical architecture

Modules should correspond to domain concepts or adapter boundaries. Do not add
generic `utils`, `helpers`, `common`, or `misc` modules as permanent dumping
grounds. If a concept cannot be named, keep it near its only caller until its
role becomes clear. When a file becomes difficult to scan, split by cohesive
responsibility, not arbitrary line count.

Keep the stable architectural map in architecture documentation and the local
mechanics next to code. As Aleksey Kladov argues in
[`ARCHITECTURE.md`](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html),
document boundaries and invariants—especially important absences such as “the
domain does not depend on transport”—rather than duplicating every implementation
detail.

## Required workflow for coding agents

An agent MUST follow this loop for Rust changes.

### 1. Establish scope and evidence

- Read this file, `AGENTS.md`, `Cargo.toml`, the nearest module declarations,
  the implementation being changed, and its separate tests.
- Inspect `git status` before editing. Preserve unrelated user work and changes
  in nested repositories.
- Search for every definition, constructor, conversion, trait implementation,
  serialization name, and caller affected by a proposed type or API change.
- Identify whether the behavior is a public Rust API, persisted representation,
  protocol contract, security boundary, or hot path.
- For performance work, capture a baseline before modifying the implementation.

### 2. State the design before coding

For non-trivial changes, write down in working notes or the eventual module
documentation:

- valid states and forbidden states;
- ownership and lifetime of important values;
- effect boundaries and failure variants;
- expected concurrency and cancellation behavior;
- compatibility constraints;
- performance hypothesis and metric, when relevant.

If these cannot be stated plainly, the implementation is not ready to become
generic or type-level.

### 3. Choose the representation

Use this progression:

1. A concrete pure function.
2. A semantically named `struct`, `enum`, or newtype.
3. A fallible smart constructor or boundary conversion.
4. A generic function if the same algorithm is genuinely independent of type.
5. A trait if behavior has multiple meaningful implementations or forms a real
   capability boundary.
6. A witness or typestate if possession or operation order is itself an
   important invariant.
7. A macro only when Rust's function and type systems cannot express repeated
   syntax acceptably.

Do not start at step 6 because the task asks for “advanced Rust.” Advanced Rust
is the judgment to stop at the simplest sufficient step.

### 4. Design tests first, in separate files

- Identify examples, boundaries, negative cases, round trips, and algebraic
  laws before implementation.
- Put all `#[test]`, `#[tokio::test]`, fixtures, fake implementations, and test
  helpers in separate files as specified below.
- For a regression, add a test that fails for the old behavior and names the
  violated contract.
- For a type-level guarantee, add a compile-fail test when ordinary runtime
  tests cannot prove the API rejects misuse.

### 5. Implement a complete vertical slice

- Make the smallest cohesive change that establishes the invariant end to end.
- Do not leave duplicate old and new policy paths without a migration plan.
- Do not use `unwrap`, a wildcard match, a string comparison, a clone, or an
  `allow` attribute to silence a design problem.
- Keep refactoring behavior-preserving and separate it conceptually from a
  behavior change. Tests should make the distinction visible.

### 6. Validate proportionally

At minimum, run the repository's CI-equivalent Rust gates:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

For public API or documentation changes, also run:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test --doc
```

If features are introduced, exercise every supported feature combination that
can change behavior; at minimum include no-default-features, default features,
and all features where applicable. If a performance-sensitive path changes,
build and benchmark an optimized executable with a locked dependency graph:

```bash
cargo build --release --locked
```

Run targeted Miri, property, fuzz, Loom, or integration checks when their risk
categories apply. Never claim a benchmark win from a debug build.

### 7. Review the diff as a maintainer

- Read the final diff, not only the final files.
- Confirm no production file contains a test body.
- Confirm error sources and security distinctions survived translation.
- Confirm no public visibility, dependency, feature, allocation, lock, clone,
  or unsafe block was added accidentally.
- Confirm docs describe why and invariants, not a stale narration of code.
- Confirm `git status` contains only intended changes.

## Model the domain with algebraic data types

Rust `struct`s are product types (“this and that”); data-carrying `enum`s are sum
types (“this or that”). Together they are algebraic data types (ADTs). This is
the default vocabulary for QueryGraph's composable representation.

Manish Goregaokar's explanation of
[sum and product types](https://manishearth.github.io/blog/2017/03/04/what-are-sum-product-and-pi-types/)
offers a practical design test: every independent field multiplies the number
of representable states. Several booleans and optional fields can therefore
create a large state space containing combinations the domain never intended.

### Use one enum variant per meaningful alternative

Reject this shape:

```rust
struct ScanResult {
    allowed: bool,
    rows: Option<Vec<Row>>,
    denial_reason: Option<String>,
    backend_error: Option<String>,
}
```

It permits contradictory values: denied rows, an allowed denial reason, two
failure modes at once, or no outcome at all. Prefer:

```rust
enum ScanOutcome {
    Granted(AuthorizedRows),
    Denied(PolicyDenial),
    Unavailable(SourceFailure),
}
```

Each variant carries exactly the data valid in that state. An exhaustive match
makes every consumer confront new states during compilation.

### ADT rules

- Use a `struct` when every field is part of one coherent value.
- Use an `enum` when exactly one alternative is active.
- Put variant-specific data inside the variant, not in parallel optional fields.
- Use `Option<T>` only for genuine, unremarkable absence.
- Use `Result<T, E>` for success versus failure.
- Use a domain enum when there are multiple successful outcomes, multiple
  actionable failure categories, or a denial is not a system failure.
- Prefer data-carrying variants over an enum tag plus a second lookup.
- Match exhaustively on owned internal enums. Avoid `_` and `..` when adding a
  variant should force a review. Wildcards are appropriate only at an explicitly
  forward-compatible external boundary where unknown cases have defined policy.
- Use `Infallible` when an implementation truly cannot fail but must satisfy a
  generic error shape. Do not invent an impossible custom error variant.
- Keep domain enums semantic. Do not mirror every transient state of an SDK.

QueryGraph already demonstrates the direction with `LakehouseSource`,
`LakehouseDataType`, `Action`, and typed stack errors. Continue replacing new
status strings, sentinel values, and boolean matrices with domain ADTs.

### Avoid primitive obsession

If two values share a primitive representation but are not interchangeable,
give them distinct types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CatalogIdentity(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantDigest(String);
```

This applies especially to:

- DIDs, catalog identities, grant IDs, policy IDs, run IDs, stable IDs, hashes,
  schema names, table names, and capability names;
- seconds versus milliseconds, row counts versus byte counts, and offsets
  versus lengths;
- raw input, canonical input, verified data, and authorized data.

A type alias does not create a distinction. `type GrantId = String` documents
intent but still accepts every `String`; use a newtype when mixing values would
be a bug.

### Parse, do not repeatedly validate

Keep invariant-bearing fields private and expose a fallible constructor:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaName(String);

impl SchemaName {
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidSchemaName> {
        let value = value.into();
        validate_schema_name(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

After construction, code accepting `&SchemaName` must not validate the same
rules again. The constructor is the single semantic source of truth. This is
the static-enforcement preference in the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/dependability.html)
and the central theme of Will Crichton's
[Type-Driven API Design in Rust](https://willcrichton.net/rust-api-type-patterns/introduction.html).

At untrusted boundaries, deserialize or parse a wire DTO, then `TryFrom` it into
the domain type. Do not derive `Deserialize` directly for a type if doing so can
bypass its invariant. If direct deserialization is desirable, use a validated
`try_from` representation or a custom implementation that calls the same smart
constructor.

### Booleans are for facts, not modes

A boolean field is appropriate for an independent binary fact with obvious
meaning, especially a local calculation. A boolean parameter or a cluster of
boolean fields often hides an unnamed enum.

Prefer:

```rust
enum AuthenticationMode {
    Optional,
    Required,
}
```

over `build_router(require_auth: bool)`, especially in public or long-lived
APIs. Prefer `is_empty`, `is_authorized`, or `contains` for predicates; prefer a
typed outcome when callers need to know why the answer is false.

### Algebraic laws are part of a representation

When an operation is intended to be associative, commutative, idempotent,
invertible, order-preserving, or identity-preserving, document that law and
property-test it. Examples include:

- canonicalization is idempotent;
- parse/serialize round trips preserve semantics;
- graph projection order does not alter a digest when the format promises
  canonical ordering;
- merging independent evidence is associative if the API claims it;
- authorization never increases through intersection;
- verification followed by rendering does not mutate verified content.

Do not call an operation `merge`, `normalize`, `canonicalize`, or `set` while
leaving its algebraic behavior ambiguous.

## Functional core, imperative shell

Rust is multi-paradigm, not a pure functional language. Use functional design
where it improves composition and reasoning, and use explicit imperative code
where stateful control flow is clearer. The goal is a pure decision core with a
small effectful shell, not Haskell syntax recreated through traits and macros.

### Separate decisions from effects

Organize workflows as typed stages:

```text
wire bytes
  -> parse
  -> validate / verify
  -> normalize
  -> plan (pure)
  -> authorize (explicit capability)
  -> execute (effectful adapter)
  -> report / serialize
```

Pure stages SHOULD:

- depend only on their arguments;
- return values rather than mutate globals;
- make clocks, randomness, identity, policy, and environment explicit inputs;
- be deterministic for the same inputs;
- return typed failures rather than log or terminate;
- avoid I/O, locks, task spawning, and hidden caches.

Effectful stages SHOULD be narrow adapters that acquire input, invoke the core,
and commit or emit an already-decided result. A database transaction may still
contain decision logic required for atomicity, but factor the deterministic
decision into a function that can be tested independently.

### Make effects visible through capabilities

Pass narrow capabilities instead of a giant application context or global
singleton. A trait is justified when it represents behavior such as reading a
clock, loading a governed source, committing an application, or checking
authority:

```rust
trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

fn validate_expiry<C: Clock>(clock: &C, grant: &Grant) -> Result<(), Expired> {
    validate_expiry_at(clock.now(), grant)
}

fn validate_expiry_at(
    now: DateTime<Utc>,
    grant: &Grant,
) -> Result<(), Expired> {
    (now < grant.expires_at()).then_some(()).ok_or(Expired)
}
```

The first function makes the clock effect explicit and replaceable; the second
is the pure decision function. Prefer passing the already acquired value deeper
into the core instead of letting every layer call a clock capability.

For an internal function with only one implementation, a concrete `&TestClock`
or a function parameter may be simpler than a trait. Do not create an interface
for every dependency by reflex.

### Prefer transformations over ambient mutation

- Take `&T` for observation, `&mut T` for a localized in-place update, and `T`
  when a transition consumes the old state.
- Return the new value or a typed change set from planning code.
- Make database and filesystem mutation happen after validation and
  authorization, at one visible commit point.
- Use RAII guards for resources whose release must follow lexical scope.
- Never hide a write in a getter, conversion, formatter, `Display`, iterator,
  equality implementation, or serialization implementation.

Mutation is not forbidden. A local `Vec` built with `push`, an in-place parser,
or a clearly owned state machine can be simpler and faster than cloning values
through a chain. The concern is aliasing and reach, not the spelling `mut`.

### Use the standard compositional vocabulary

Use the combinator that makes the control flow obvious:

- `map` transforms a present/successful/item value.
- `and_then` sequences a dependent optional or fallible operation.
- `ok_or_else` changes meaningful absence into a lazily constructed error.
- `map_err` translates at a real error boundary; it is not a place to erase
  categories into strings.
- `transpose` cleanly changes `Option<Result<T, E>>` into
  `Result<Option<T>, E>` (or the iterator analogue).
- `collect::<Result<Vec<_>, _>>()` traverses while failing fast.
- `try_fold` combines a stream with fallible or short-circuiting state.
- `filter_map` selects and transforms in one pass when dropping values is the
  explicit semantics.
- `then_some` and `then` are suitable for small, obvious conditional values.

Name an intermediate value when a chain mixes concerns or requires mental type
inference. A five-line pipeline with domain names is often more functional—in
the sense of compositional and reasoned—than a one-line chain of twelve
adapters.

### Iterator or loop?

The Rust Book emphasizes that closures and iterators are idiomatic and can
compile as efficiently as hand-written loops. Prefer iterator pipelines when
they express a linear transformation without hidden effects.

Prefer a loop when:

- the algorithm is a state machine;
- several accumulators interact;
- early exits have distinct outcomes;
- mutation is the clearest ownership model;
- an iterator chain needs side effects in `map` or obscure captures;
- borrow-checker workarounds make the chain harder to understand;
- profiling shows a concrete code-generation issue.

Do not use `map` only for side effects; use `for_each` sparingly and usually a
`for` loop. Do not use `inspect` for required behavior. Do not index a slice in
a range loop when iterating or zipping the slices states the invariant and lets
the compiler remove bounds checks.

Rust does not guarantee tail-call optimization. Use an iterative algorithm for
unbounded recursion or prove and test a small recursion bound.

### Higher-order functions and closures

Closures are the first choice for local policy, callbacks, comparators, and
small strategies. A named function is better when the behavior is reused,
tested independently, or carries domain meaning. A trait is better when the
behavior forms a stable capability with multiple implementations.

Do not introduce currying, lenses, HKT emulation, a generic `Monad` hierarchy,
or function-composition macros. Rust's standard `Option`, `Result`, `Iterator`,
closures, ownership transitions, and ADTs provide the useful portion of that
vocabulary with better diagnostics and ecosystem interoperability.

## Type-driven design without type gymnastics

Use type-level programming to encode facts the compiler can usefully enforce.
The public experience matters more than the cleverness of the implementation.

### Newtypes and smart constructors

Use a newtype when it provides at least one of:

- a semantic distinction;
- an invariant with a private representation;
- controlled construction;
- a focused trait implementation;
- an abstraction boundary that can change representation;
- prevention of accidental argument reordering.

Implement applicable common traits deliberately (`Debug`, equality, ordering,
hashing, `Display`, `FromStr`, `TryFrom`, `AsRef`). Do not implement `Deref` to
make a domain newtype pretend to be its backing primitive; `Deref` is for smart
pointer semantics. Expose a named accessor instead.

Do not make every string a newtype. The distinction must prevent a plausible
bug, establish an invariant, or define a durable API concept.

### Witnesses and capabilities

A witness is an unforgeable value proving a fact. This is especially suitable
for QueryGraph's authorization and verification flows:

```rust
pub struct VerifiedGrant {
    grant: Grant,
    subject: VerifiedSubject,
    // Private evidence required to establish the proof.
}

pub fn reveal(
    source: &GovernedSource,
    grant: &VerifiedGrant,
) -> Result<AuthorizedRows, RevealError> {
    // Possession of `VerifiedGrant` is a required proof, not a convention.
    source.reveal_with(grant)
}
```

Witness rules:

- Fields are private; construction is possible only through the verifier that
  establishes the fact.
- A witness is tied to the subject, resource, policy version, scope, and time it
  proves. Do not use a context-free `Authorized` marker for a contextual fact.
- Borrow a reusable witness; consume a single-use capability.
- Do not serialize and later trust a witness as though the Rust type survived
  the trust boundary. Deserialize evidence, then verify it to mint a new local
  witness.
- A zero-sized witness is appropriate only if no runtime evidence must be
  retained and its constructor remains unforgeable.

See Will Crichton's discussion of
[witnesses](https://willcrichton.net/rust-api-type-patterns/witnesses.html) and
[guards](https://willcrichton.net/rust-api-type-patterns/guards.html).

### Typestate

Typestate encodes operation order in a type parameter or distinct state types:

```rust
pub enum Unverified {}
pub enum Verified {}

pub struct Bundle<State> {
    payload: BundlePayload,
    state: std::marker::PhantomData<State>,
}

impl Bundle<Unverified> {
    pub fn verify(self) -> Result<Bundle<Verified>, VerificationError> {
        verify_payload(&self.payload)?;
        Ok(Bundle {
            payload: self.payload,
            state: std::marker::PhantomData,
        })
    }
}

impl Bundle<Verified> {
    pub fn import_plan(&self) -> ImportPlan {
        plan_import(&self.payload)
    }
}
```

Use typestate when all of these are true:

- the state machine is small and stable;
- illegal operation order is an important correctness or security bug;
- transitions naturally consume or uniquely borrow the value;
- most callers know the state statically;
- the improvement is visible at call sites;
- compiler errors remain understandable;
- heterogeneous storage does not constantly erase the state.

Prefer a runtime `enum` when state is loaded from storage, selected dynamically,
stored heterogeneously, has many independent dimensions, or must be inspected
and changed repeatedly in one collection. Typestate should not cause a
combinatorial product like `Thing<Auth, Open, Cached, Retried, Version, Mode>`.
Collapse interacting dimensions into one semantic state enum or keep truly
runtime state at runtime.

Cliff Biffle's
[typestate guide](https://cliffle.com/blog/rust-typestate/) and Will Crichton's
[typestate chapter](https://willcrichton.net/rust-api-type-patterns/typestate.html)
show the benefits and mechanics. Apply them selectively.

### `PhantomData` is semantic, not decoration

`PhantomData` affects variance, drop checking, and auto traits such as `Send`
and `Sync`, despite having zero size. Use it only when the type logically owns,
borrows, or is parameterized by something not present in a field. Choose its
form deliberately and audit auto-trait behavior. Consult the
[Rustonomicon's `PhantomData` chapter](https://doc.rust-lang.org/nomicon/phantom-data.html)
for anything beyond a simple private typestate marker.

If a state contains actual runtime data, store the state value instead of a
phantom marker. Do not add `PhantomData` until the semantics of ownership,
variance, and thread safety can be explained.

### Traits, associated types, GATs, and HRTBs

Use an associated type when each implementation has one natural related type:

```rust
trait SourceVerifier {
    type Evidence;
    type Error;

    fn verify(&self, input: RawSource) -> Result<Self::Evidence, Self::Error>;
}
```

Use a generic method or trait parameter when one implementation intentionally
supports many caller-selected types. Place bounds on the narrowest method or
`impl` that needs them rather than on the data structure itself.

Generic associated types (GATs) are appropriate for a real family of types,
especially an output borrowing from `self` at a caller-selected lifetime. A
higher-ranked trait bound (`for<'a>`) is appropriate when a callback must work
for every borrow lifetime rather than one captured lifetime. Hide complex bounds
behind a well-named private trait or type when possible.

Before using a GAT or HRTB, require all of:

- it removes an allocation, clone, or unsound/awkward lifetime workaround;
- the relationship cannot be expressed clearly with an ordinary lifetime or
  associated type;
- the public compiler error is acceptable;
- the abstraction has compile-fail and runtime tests;
- rustdoc renders a usable API.

Do not emulate higher-kinded types with deeply recursive traits, type-level
lists, or macro-generated impl grids. Do not use unstable specialization to
remove a small duplication. Stable, explicit code wins.

### Const generics

Use const generics when a value is genuinely a compile-time dimension and the
dimension changes representation or safety—for example a fixed digest width or
an array-backed structure. Do not promote ordinary runtime limits, tenant
configuration, batch size, or retry count into type parameters. Doing so
multiplies monomorphizations and makes configuration need recompilation.

### Sealed traits

Seal a public trait when downstream implementations would prevent safe API
evolution or when the trait is only an extension mechanism for types owned by
QueryGraph. Document that it is sealed. Leave a trait open when third-party
implementation is an intentional part of the contract. See the
[API Guidelines on future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html).

### The type-gymnastics stop rule

Replace a type-level design with a simpler ADT or runtime check if two or more of
these are true:

- the type signature is harder to explain than the invariant;
- a normal misuse produces pages of trait-solver output;
- implementation details leak into most callers;
- small domain changes require many impls or macro arms;
- compile time or binary size materially increases;
- common storage requires immediate `dyn Any`, boxing, or state erasure;
- the design relies on undocumented auto-trait, variance, coherence, or layout
  behavior;
- only the original author can confidently modify it;
- a private smart constructor plus enum enforces nearly the same property.

Advanced abstraction is successful when downstream code becomes boring.

## DRYness and abstraction design

DRY means each piece of domain knowledge has one authoritative expression. It
does not mean every repeated token sequence must be factored into indirection.

### Eliminate semantic duplication

There MUST be one source of truth for:

- validation and canonicalization rules;
- security and authorization decisions;
- protocol constants and version selection;
- hash/signature payload formation;
- conversion between a wire contract and a domain contract;
- retry/CAS classification;
- SQL identifier quoting and path safety;
- benchmark workload generation;
- shared test fixtures and assertions of the same invariant.

If Rust, Python, TypeScript, or an external component must implement the same
wire rule, define the rule in a shared schema/specification or cross-language
conformance fixtures. Copying an implementation into each language is not a
single source of truth merely because the copies currently agree.

### Do not abstract accidental similarity

Two functions with similar control flow may encode different policies that will
evolve independently. Keep them separate until the shared concept can be named
without `mode` flags or callback soup. A good abstraction:

- has a domain name;
- gives callers fewer states and decisions to manage;
- centralizes an invariant, algorithm, or protocol;
- has one reason to change;
- does not require unrelated type parameters;
- improves or preserves diagnostics;
- can be tested through its contract;
- does not hide important allocation, I/O, locking, or failure behavior.

Kladov's
[“Concrete Abstraction”](https://matklad.github.io/2020/08/15/concrete-abstraction.html)
is the relevant warning: abstraction has cognitive and monomorphization cost.
The response is not to avoid abstraction, but to demand that it compress real
knowledge.

### Extraction ladder

When removing duplication, try in order:

1. Name an intermediate value.
2. Extract a concrete private function.
3. Extract a semantic data type or configuration value.
4. Implement a standard trait (`From`, `Iterator`, `Display`, `AsRef`) when its
   established meaning is exact.
5. Parameterize the genuinely variable value or behavior.
6. Introduce a local trait for multiple implementations or a capability seam.
7. Introduce a macro only for syntax or a mechanical impl matrix that functions
   and traits cannot express cleanly.

Do not jump from copied blocks directly to a procedural macro.

### Trait design

Introduce a trait when at least one is true:

- two meaningful implementations exist or are being added now;
- application logic depends on a capability with an external and fake/local
  implementation;
- an open extension point is part of the product contract;
- static polymorphism removes measured overhead in a hot reusable algorithm;
- dynamic polymorphism intentionally erases a heterogeneous set.

Do not create a trait solely to mock one pure function, to wrap every struct, or
because another language would use an interface. Prefer a closure for one local
operation and a concrete type for one implementation.

Keep traits cohesive and small, but not atomized into one-method fragments that
always travel together. Required methods state the semantic minimum; provided
methods may derive conveniences from that minimum. Decide whether a trait must
be dyn-compatible before publishing it, and use `where Self: Sized` for generic
conveniences that need not be callable through `dyn Trait`.

### Static dispatch, dynamic dispatch, or enum

Choose explicitly:

- An `enum` is best for a closed, known set of alternatives. It gives exhaustive
  matching, compact storage, and no vtable.
- A generic parameter / `impl Trait` is best for hot reusable algorithms, small
  combinators, and zero-overhead caller-selected implementations.
- `dyn Trait` is best for open heterogeneous sets, stable orchestration
  boundaries, plugins, reducing monomorphization, or keeping large cold paths
  out of generic code.

Static dispatch is not universally faster: it can increase compilation work,
binary size, instruction-cache pressure, and downstream monomorphization.
Dynamic dispatch has a predictable indirect call and often an allocation only
because of the chosen owner (`Box`, `Arc`), not because `dyn` itself requires
one in every form. Measure hot paths.

For ergonomic generic public inputs, use a thin generic wrapper that immediately
normalizes to a concrete internal function:

```rust
pub fn load(path: impl AsRef<Path>) -> Result<Model, LoadError> {
    load_path(path.as_ref())
}

fn load_path(path: &Path) -> Result<Model, LoadError> {
    // Substantial non-generic implementation.
    load_model_from_path(path)
}
```

This follows Kladov's guidance on
[fast Rust builds](https://matklad.github.io/2021/09/04/fast-rust-builds.html):
keep generic boundary code thin so consumers do not repeatedly monomorphize a
large body.

### Macros

Use ordinary functions, traits, derives, and build-time data before custom
macros. A declarative macro is justified for repeated syntax, tuple/arity impls,
or a small DSL whose output and errors remain clear. A procedural macro is a
separate compiler component and must clear a high bar.

Every custom macro MUST have:

- documentation of accepted syntax and expansion semantics;
- tests for valid input, boundary cases, and compiler diagnostics;
- hygienic paths using `$crate` where applicable;
- no hidden I/O or environment-dependent generation;
- a reason a function, derive, or data file is insufficient.

Do not use a macro to hide control flow, authorization, allocation, unsafe code,
or SQL construction.

### Performance-motivated duplication

Duplication is allowed only when deduplication causes a demonstrated material
regression that cannot be fixed cleanly. The duplicate paths MUST:

- be isolated behind one semantic API;
- cite the benchmark and target workload in a nearby comment or performance
  document;
- share conformance/property tests;
- state which implementation is authoritative;
- have a removal condition if compiler or dependency behavior changes.

“Might inline better” is not evidence.

## Modules and public APIs

### Design modules around invariants

A module owns the invariants of its private fields and constructors. Keep all
code capable of violating those invariants small enough to audit. This privacy
boundary is especially important for verified values and any future unsafe
abstraction.

Good module contents usually include:

- domain type definitions;
- their smart constructors and invariant-preserving methods;
- closely related conversions;
- a private pure algorithm;
- a separate test module file.

Split wire DTOs, storage adapters, and transport mappings when they would make
the domain module depend outward.

### Visibility is a commitment

- Begin with private.
- Use `pub(super)` for a parent-owned collaboration.
- Use `pub(crate)` for an intentional internal service.
- Use `pub` only for a documented external contract.
- Avoid public fields on invariant-bearing types.
- Avoid `pub use module::*` in public facades. Re-export named, intentional
  items so additions do not silently expand the API.
- Do not widen visibility only to make an integration test compile. Use a child
  unit-test module in a separate file or test through the public API.

Before changing a public signature, inspect downstream use and treat types,
trait bounds, auto traits, error variants, serialization, `Send`/`Sync`, and
panic behavior as compatibility concerns.

### Input and output conventions

- Borrow inputs when the function only observes them: `&str`, `&Path`, `&[T]`,
  and `&T`, not `&String`, `&PathBuf`, or `&Vec<T>`.
- Take ownership when the function stores, consumes, normalizes in place, or
  transitions the value.
- Use `impl Into<String>` sparingly for true ownership-taking convenience; do
  not make every internal function generic for call-site aesthetics.
- Return owned domain values when ownership transfers.
- Return `impl Iterator` for a lazy internal/public traversal when it preserves
  flexibility and lifetimes remain understandable.
- Return a slice when exposing stable contiguous borrowed data.
- Do not allocate a `Vec` merely to return something already iterable unless a
  snapshot, ownership boundary, sort, or reuse requires materialization.
- Use `Cow` only when both borrowed and owned outcomes occur meaningfully. It is
  not a default substitute for deciding ownership.

### Constructors and builders

- Use `new` for the obvious primary valid construction.
- Use semantic constructors (`parse`, `open`, `verify`, `from_bundle`) when work
  or failure is meaningful.
- Use a builder for many optional parameters, staged configuration, or a value
  whose final validation happens once at `build`.
- Do not use a builder for a three-field value with no defaults.
- A builder must not permit `build` to create an invalid domain value.
- Use typestate builders only when required fields or ordering are critical and
  diagnostics stay humane; otherwise validate at `build`.

### Standard traits and conventions

Follow the [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html):

- implement common traits where semantics are honest;
- use `From`/`Into` for infallible, lossless, unsurprising conversions;
- use `TryFrom`/`FromStr` for fallible parsing or validation;
- use `as_`, `to_`, and `into_` according to borrowing/allocation/ownership;
- name iterator methods `iter`, `iter_mut`, and `into_iter`;
- make `Default` the unsurprising default, not merely any constructible value;
- mark important returned plans, guards, receipts, or transformations
  `#[must_use]` when silently dropping them is likely a bug.

Do not implement a standard trait approximately. A surprising `From`, lossy
`Display`, order inconsistent with equality, or clone that changes identity is
worse than a named method.

### Semver-aware types

- Consider `#[non_exhaustive]` for public enums/structs expected to grow across
  crate boundaries, understanding that it weakens downstream exhaustive match.
- Keep fields private when representation evolution matters.
- Do not add unnecessary trait bounds to a public data type; consumers pay them
  and adding/removing bounds can be breaking.
- Seal traits not intended for downstream implementation.
- Treat serialized forms separately from Rust semver. A private Rust type can
  still encode a public persisted contract.

## Ownership, borrowing, and resources

Ownership is architecture. A signature communicates who may retain, mutate,
share, and release a value.

### Choose the narrowest ownership mechanism

| Need | Prefer | Cost / warning |
| --- | --- | --- |
| Observe during a call | `&T` | Borrow must not escape. |
| Mutate exclusively | `&mut T` | Keep scope narrow. |
| Transfer or consume | `T` | Makes state transitions explicit. |
| Optional owned value | `Option<T>` | Absence must be semantically valid. |
| Single heap owner | `Box<T>` | Allocation and indirection. |
| Shared single-thread owner | `Rc<T>` | Refcount; cycles possible. |
| Shared cross-thread owner | `Arc<T>` | Allocation and atomic refcount. |
| Runtime borrow checking | `RefCell<T>` | Dynamic checks; may panic. |
| Shared mutation | `Mutex<T>` / `RwLock<T>` | Contention and deadlock risk. |
| Copyable shared scalar state | atomics | Memory-order proof required. |

Do not begin with `Arc<Mutex<T>>`. First ask whether one task can own the state,
whether messages can express operations, whether callers can pass `&mut`, or
whether the data can be immutable and replaced wholesale.

Manish Goregaokar's writing on
[shared mutability](https://manishearth.github.io/blog/2015/05/17/the-problem-with-shared-mutability/)
and [wrapper guarantees](https://manishearth.github.io/blog/2015/05/27/wrapper-types-in-rust-choosing-your-guarantees/)
is the model: select wrappers by the exact guarantee and cost needed, and pay
attention to where in a composition mutation is permitted.

### Cloning

`clone` is an explicit ownership operation, not a code smell by itself. It may
be the clearest correct choice for a small value, snapshot, independent request,
or cheap shared handle. It is a smell when used reflexively to satisfy the
borrow checker or when its cost is unknown.

Before cloning:

1. Shorten the borrow scope.
2. Reorder work so a borrow ends before mutation.
3. Borrow a field rather than the whole object.
4. Split a struct into independently borrowable components if that matches the
   domain.
5. Use `mem::take`, `Option::take`, or a consuming transition when ownership is
   truly moving.
6. Clone deliberately if it remains the clearest model.

Do not contort cold code to remove an unmeasured clone. Profile allocation hot
spots first, as the
[Rust Performance Book](https://nnethercote.github.io/perf-book/heap-allocations.html)
recommends.

### RAII and guards

Use RAII for locks, temporary files, transactions, spans, subscriptions, and
other lexical resources. A guard should both prove acquisition and mediate
access. Keep guards short-lived and do not leak them into unrelated layers.

Do not rely on `Drop` for a security or memory-safety invariant that safe code
can violate with `mem::forget`, process termination, cycles, or leaks. `Drop`
is appropriate for best-effort resource release; provide explicit `commit`,
`close`, or `shutdown` when the caller must observe failure.

### Borrowing in data structures

Owned domain models are usually simpler at application and persistence
boundaries. Add lifetimes to stored domain types only when borrowing has a
meaningful measured benefit and the owner relationship is stable. Avoid
self-referential designs; store offsets/indices, use an owning representation,
or rely on a proven library abstraction.

Do not return a borrow tied to a lock guard, temporary deserialization buffer,
or adapter session unless that dependency is the intended API.

## Errors and failure semantics

Errors are ADTs and part of the API. Preserve categories callers need for
policy, retry, status mapping, observability, and security.

### Typed errors in reusable code

Library and domain boundaries SHOULD expose focused error enums, usually with
`thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum VerifyBundleError {
    #[error("manifest hash does not match the bundle")]
    ManifestMismatch,

    #[error("catalog graph is invalid")]
    InvalidGraph(#[source] GraphValidationError),

    #[error("verification service is unavailable")]
    Unavailable(#[source] SourceError),
}
```

Use `anyhow` at executable/orchestration boundaries where the operation will be
reported rather than programmatically matched. It is also reasonable inside a
narrow adapter whose caller intentionally treats all implementation failures as
one service category, but map to that category before crossing a policy/public
boundary.

### Error rules

- A recoverable condition returns `Result`; a genuine optional lookup returns
  `Option`; a bug/invariant violation may panic.
- Denial, conflict, not-found, malformed input, timeout, cancellation, and
  backend outage are distinct when callers act differently.
- Use `?` for propagation and `From`/`#[from]` only when the conversion preserves
  the intended abstraction.
- Add context at semantic boundaries: which bundle, table, operation, or
  endpoint—not at every stack frame.
- Preserve sources with `#[source]` or `anyhow::Context`.
- An error's `Display` should describe that layer and MUST NOT duplicate the
  source text if the source is also returned through `Error::source`; the Rust
  Error Handling Project Group documents why duplicate chains produce unusable
  reports.
- Do not match `Display` strings. Match typed variants or error codes.
- Do not erase a typed error into a string before logging/status translation.
- Do not log and return the same error at every layer. Report once at the
  handling boundary; attach structured context while propagating.
- Do not swallow an error with `.ok()`, `unwrap_or_default`, or `let _ =` unless
  losing it is the documented semantics.
- Redact secrets, bearer tokens, signed payload internals, private rows, and
  backend-controlled text at trust boundaries.

### Panics, `unwrap`, and `expect`

Production code MUST NOT use `unwrap` for input, I/O, serialization, time,
network, lock, channel, database, or other environmental outcomes.

`expect` is permitted only when the condition is a local invariant established
immediately by construction and returning an error would falsely imply a
runtime contingency. Its message must state the invariant (“constant JSON value
is serializable”), not “should work.” Prefer an infallible API or a type that
proves the invariant when available.

Tests may use `unwrap` for setup, but `expect` with a behavior-specific message
usually produces a better failure. Tests of errors should match exact variants,
not merely call `is_err`.

Do not use panic as ordinary branching. Public documentation MUST include a
`# Panics` section for reachable panic conditions.

### Fail closed

Authorization and verification failures must fail closed without collapsing
important operator diagnostics. Model at least:

- an authoritative denial;
- invalid or stale evidence;
- provider/service unavailability;
- malformed caller input;
- internal invariant failure.

Do not let adapter-controlled text decide which category applies. Translate
from typed adapter errors. Do not expose protected data in any error path before
authorization succeeds.

## Async and concurrency

Async is an I/O concurrency tool, not a marker for “modern” code. Keep pure and
CPU-only functions synchronous. An `async fn` compiles to a state machine; every
value live across `.await` can become part of that future.

### Async design rules

- Make async boundaries correspond to actual waiting.
- Do not call blocking filesystem, network, database, subprocess, compression,
  or heavy CPU work on a Tokio worker thread.
- Use `spawn_blocking` for bounded blocking I/O, a dedicated thread for a
  permanent blocking resource loop, and a bounded CPU pool such as Rayon for
  substantial parallel computation.
- Scope and drop large temporaries before `.await` when they are not needed
  afterward; this can reduce future size.
- Do not hold a borrow, lock guard, database transaction guard, or temporary
  buffer across `.await` unless the API explicitly requires it and cancellation
  behavior is understood.
- Prefer structured ownership of tasks. Every spawned task needs an owner,
  shutdown path, and observed `JoinHandle` result, or a documented process-long
  lifetime.
- Never spawn-and-forget work that commits data, releases authorization state,
  or carries a response the caller assumes completed.

Alice Ryhl's
[“Async: What is blocking?”](https://ryhl.io/blog/async-what-is-blocking/)
is the practical reference. Tyler Mandry's series on
[how Rust optimizes async/await](https://tmandry.gitlab.io/blog/posts/optimizing-await-1/)
explains the generated state machine and why live values matter.

### Shared state

- Use a synchronous mutex for a short, low-contention, non-async critical
  section, and ensure its guard's lexical scope ends before `.await`.
- Use Tokio's async mutex only when a guard genuinely must survive waiting; it
  is more expensive, not the default mutex for async code.
- For I/O resources or complex state transitions, prefer one owning task and a
  typed message enum.
- Use bounded channels by default so capacity expresses backpressure.
- Choose and document overload behavior: wait, reject, shed, coalesce, or drop.
- Consider sharding only after measuring lock contention and confirming keys are
  independent.
- Do not hold one lock while acquiring another without a documented total order.

The [Tokio shared-state guide](https://tokio.rs/tokio/tutorial/shared-state)
and Alice Ryhl's
[actor pattern](https://ryhl.io/blog/actors-with-tokio/) provide the baseline.

### Cancellation, timeouts, and shutdown

Every await can be a cancellation point. For an operation with externally
visible mutation, define whether cancellation before, during, or after commit is
safe. Prefer transactional or idempotent operations and explicit receipts.

- Put timeouts at the layer that knows the latency budget.
- Distinguish timeout from authoritative denial or backend failure.
- Make retries bounded and cancellation-aware.
- Use stable operation/idempotency keys for retryable commits.
- Close channels and drain or reject pending work deliberately.
- Signal shutdown, stop accepting work, await owned tasks, and report failures.
- Test cancellation at each externally meaningful await boundary for critical
  workflows.

### Concurrency bounds

Unbounded `join_all`, task spawning, buffering, request fan-out, and channels are
resource leaks under load. Make maximum in-flight work explicit with a semaphore,
bounded stream combinator, worker count, or channel capacity. Benchmark both
throughput and p95/p99 latency under representative concurrency.

### Atomics and lock-free code

Do not write custom atomics or lock-free structures for QueryGraph unless a
profile proves a synchronization bottleneck and existing well-maintained
primitives cannot solve it. Any atomic algorithm requires:

- a written invariant and happens-before argument;
- justification for each memory ordering;
- Loom model tests;
- Miri where supported;
- architecture-aware benchmarks;
- review by someone experienced with Rust's memory model.

Defaulting everything to `SeqCst` does not repair an invalid algorithm; choosing
`Relaxed` because it benchmarks faster is not a proof. Mara Bos's
[Rust Atomics and Locks](https://marabos.nl/atomics/) is required reading before
such work.

## Performance engineering

High-level Rust and excellent performance are allies when abstractions express
information the optimizer can use. They become enemies when an abstraction
causes unnecessary allocation, indirection, cloning, contention,
monomorphization, or repeated work. Determine which by measurement.

### Performance workflow

1. Define the user-visible workload and metric.
2. Capture a reproducible optimized baseline.
3. Profile to locate CPU, allocation, I/O, contention, or scheduling cost.
4. Form one falsifiable hypothesis.
5. Change one relevant design dimension.
6. Run correctness gates and compare benchmark distributions.
7. Inspect generated code/type sizes only when the profile points there.
8. Keep the change only if the gain is meaningful relative to complexity and
   noise.
9. Record environment, command, data, concurrency, samples, and before/after
   results.

Follow Nicholas Nethercote's
[Rust Performance Book](https://nnethercote.github.io/perf-book/) and its core
rule: profile before optimizing.

### Benchmark design

Use microbenchmarks for pure kernels and integration/load benchmarks for
end-to-end behavior. A benchmark MUST:

- run an optimized production-equivalent build;
- use `std::hint::black_box` or harness facilities where optimization could
  remove the work;
- include realistic sizes and distributions, not only tiny happy paths;
- include cold/warm cache behavior if both matter;
- control setup so timed work is intentional;
- report variance or confidence, not one stopwatch sample;
- compare semantically equivalent outputs;
- include failure and contention paths when they matter in production;
- avoid live internet services unless the purpose is explicitly an external
  system benchmark and the environment is recorded.

For concurrent services, record at least throughput, p50/p95/p99 latency,
errors, retries/conflicts, CPU, and peak memory at multiple concurrency levels.
An optimization that raises median throughput while creating an unacceptable
tail or conflict rate is not automatically a win.

Use Criterion or Divan for in-process statistical benchmarks when adding a
benchmark harness, and Hyperfine or a load generator for process/end-to-end
work. Keep benchmark code under `benches/` or a dedicated benchmark package,
separate from tests and production modules.

### Optimize in the right order

Prefer improvements in this order:

1. Better algorithm or fewer passes/queries/round trips.
2. Avoid repeated parsing, validation, hashing, serialization, or conversion.
3. Batch I/O and database operations while preserving bounded memory and
   transactional semantics.
4. Improve data ownership and layout; avoid unnecessary materialization.
5. Reduce allocation and copying in measured hot paths.
6. Reduce lock scope/contention and bound task scheduling.
7. Improve cache behavior, dispatch, inlining, or vectorization.
8. Use specialized/unsafe techniques only after all safer levels are exhausted.

A single avoided network or database round trip usually dominates many local
iterator or dispatch micro-optimizations.

### Allocations and data movement

- Preallocate with `with_capacity` when a reliable size hint exists on a hot
  path.
- Stream or borrow rather than collect only when lifetime and retry semantics
  remain clear.
- Reuse buffers in measured loops; do not create mutable buffer pools globally
  without proving benefit and bounding retention.
- Avoid repeated `format!`, `.to_string()`, JSON conversion, and path/string
  round trips in hot loops.
- Use `clone_from` or reusable storage only when profiling shows allocation
  churn and the resulting code stays clear.
- `Arc::clone` is cheap relative to deep clone but still performs atomic work;
  do not scatter it without ownership reason.
- `Cow`, `SmallVec`, small-string crates, arenas, interning, and custom allocators
  are profile-driven tools, never defaults. They have branch, footprint,
  retention, and complexity tradeoffs.

### ADT and type layout performance

ADTs are normally compact and efficient, but one large enum variant determines
the size of every value. If a hot or highly replicated enum is unexpectedly
large, measure with `size_of` or `-Zprint-type-sizes` in a diagnostic nightly
run. Consider boxing a rare large variant only after comparing allocation cost
against cache/stack savings. The Performance Book's
[type-size guidance](https://nnethercote.github.io/perf-book/type-sizes.html)
is the reference.

Do not rely on default Rust field order, enum niche layout, closure layout, or
ABI. `#[repr(C)]`, `#[repr(transparent)]`, and integer enum representations are
for explicit layout contracts, not speculative speed. See the
[Rust Reference on type layout](https://doc.rust-lang.org/reference/type-layout.html).

### Iterators and bounds checks

Iterator pipelines and closures are foundational zero-cost abstractions. Prefer
slice iteration, `zip`, chunks, and exact iterators to manual indexing; they
often make bounds relationships clearer to LLVM. If a hot loop remains slow,
inspect its profile and generated assembly before replacing readable iterator
code. Huon Wilson's writing on
[closures](https://huonw.github.io/blog/2015/05/finding-closure-in-rust/) and
the Performance Book's
[iterator chapter](https://nnethercote.github.io/perf-book/iterators.html)
explain the model.

### Generics, dispatch, and compile cost

Generic code is monomorphized. That enables inlining and specialization by
concrete type but can duplicate large bodies across types, codegen units, and
crates.

- Keep generic hot adapters small.
- Move large cold/error paths into concrete non-generic functions.
- Use `dyn Trait` intentionally at cold orchestration boundaries or for open
  heterogeneous storage.
- Use an enum for a small closed implementation set.
- Check `cargo llvm-lines`, build timings, and binary size when a generic
  abstraction spreads widely.
- Do not expose a huge nested iterator/future type when an opaque return,
  newtype, or carefully chosen dynamic boundary gives a more stable API.

Aaron Turon's
[“Abstraction without overhead”](https://blog.rust-lang.org/2015/05/11/traits/)
and withoutboats'
[“Zero Cost Abstractions”](https://without.boats/blog/zero-cost-abstractions/)
capture both halves of the requirement: near-handwritten runtime performance
and an abstraction usable enough to justify itself.

### Inlining and code-generation hints

Do not add `#[inline]`, `#[inline(always)]`, `#[cold]`, branch hints, target CPU
flags, or unsafe unchecked operations by intuition.

- Cross-crate tiny generic methods are natural `#[inline]` candidates, but the
  compiler already handles many.
- `#[inline(always)]` requires benchmark and code-size evidence.
- A larger binary or degraded instruction cache can offset call removal.
- Target-specific features require portable fallback and CI/test coverage.
- LTO, codegen units, panic strategy, PGO, and allocator changes are product
  build decisions and must be benchmarked on production workloads.

### Hashing and deterministic output

Use `HashMap`/`HashSet` for general internal lookup. Use `BTreeMap`/`BTreeSet` or
explicit sorting when canonical, reproducible output is part of a digest, test,
wire contract, or audit record. Never depend on hash iteration order.

Do not replace the standard hash builder with a non-DoS-resistant hasher for
attacker-controlled keys. A faster hasher is acceptable only for trusted key
sets after measurement and explicit threat analysis.

### Caches

A cache is mutable state plus an invalidation, consistency, and memory-retention
policy. Add one only with:

- a measured repeated cost;
- an explicit key including every semantic dependency;
- bounded size/lifetime;
- invalidation behavior;
- concurrency behavior;
- hit/miss/eviction observability;
- tests for stale and adversarial cases.

Memoizing authorization, verification, time-sensitive evidence, or catalog
state without binding all authority/version/expiry inputs is a correctness bug.

### Performance exception record

When performance requires code that is less obviously idiomatic, add a concise
nearby comment or performance document containing:

- measured workload and baseline;
- result and variance;
- why the clearer implementation loses;
- invariant that must remain true;
- benchmark/test that guards it;
- conditions under which the exception should be revisited.

Do not comment routine fast code with unsupported claims such as “avoid
allocation.” Show the evidence where it can be maintained.

Andrew Gallant's
[ripgrep benchmark analysis](https://burntsushi.net/ripgrep/) is exemplary:
algorithm, realistic workload, correctness, Unicode behavior, system I/O, and
reproducibility are analyzed together. It also demonstrates a key DRY win:
putting literal-search optimization in a reusable regex engine benefits every
consumer.

## Unsafe Rust and FFI

QueryGraph-owned Rust SHOULD contain no `unsafe` code. Safe Rust is usually fast
enough, and dependencies already encapsulate low-level machinery. Do not add
unsafe merely to remove a bounds check, avoid a clone, call a convenient C API,
or work around the borrow checker.

### Approval bar

Before adding unsafe, establish all of:

- a profiler identifies a material bottleneck or an external ABI makes it
  unavoidable;
- no sound stable standard-library or established-crate abstraction suffices;
- the unsafe surface can be isolated behind one small private module;
- the safety invariant can be written precisely;
- a safe API prevents callers from violating it;
- the code has an experienced unsafe review;
- Miri, fuzz/property tests, and relevant platform tests can exercise it.

### Required unsafe documentation

Every unsafe module must state:

- validity and safety invariants;
- ownership, aliasing, initialization, provenance, alignment, and lifetime
  assumptions;
- panic, unwind, drop, and leak behavior;
- `Send` and `Sync` reasoning;
- ABI/layout assumptions;
- why safe callers cannot violate the invariant.

Every `unsafe {}` block needs a `// SAFETY:` comment explaining why each
precondition is true at that point. Every `unsafe fn` needs a rustdoc `# Safety`
section defining caller obligations. Enable and obey `unsafe_op_in_unsafe_fn` so
unsafe operations remain locally visible.

Ralf Jung's
[“The Scope of Unsafe”](https://www.ralfj.de/blog/2016/01/09/the-scope-of-unsafe.html)
is the core abstraction lesson: privacy ends the area that must uphold extra
invariants. His pointer-provenance writing and the
[Rustonomicon](https://doc.rust-lang.org/nomicon/) are required references.
Gankra's collections writing similarly shows that unsafe is for building a
small reusable safe abstraction, not application-level convenience.

### Unsafe verification

- Run `cargo +nightly miri test` on supported focused tests. Miri finding UB is
  decisive; Miri finding none is not a proof.
- Fuzz all parsers and safe entry points that can reach the unsafe core.
- Property-test boundary lengths, alignment-sensitive cases, empty/ZST values,
  panic paths, and drop behavior.
- Use Loom for custom concurrency primitives.
- Run sanitizers and platform/architecture CI where FFI or target behavior
  differs.
- Audit `Send`/`Sync`, including effects of raw pointers and `PhantomData`.

### FFI and layout

- Keep raw foreign types in an `ffi`/adapter module and convert immediately to
  safe domain types.
- Validate nullability, lengths, ownership, encoding, alignment, and integer
  conversion at the boundary.
- Specify ABI explicitly.
- Pair every allocation with its correct allocator/owner.
- Do not let unwinding cross an ABI boundary unless that ABI explicitly permits
  it and behavior is tested.
- Use `#[repr(C)]` only where a C layout contract exists and
  `#[repr(transparent)]` for a documented one-field ABI wrapper.
- Never transmute based on equal `size_of` alone.

## Serialization, persistence, and security boundaries

Serialization is a trust and compatibility boundary, not a derive convenience.

### Separate wire and domain types

Use a wire DTO when any of these differ from the domain:

- field names or tagging;
- optionality/defaulting;
- backward/forward compatibility;
- weak string representation;
- validation;
- version migration;
- secret/redacted fields;
- canonicalization rules.

Convert once into a valid domain value. Keep `serde` attributes from dictating
core architecture where a DTO would isolate the concern.

### Format rules

- Version persisted and cross-service formats intentionally.
- Use explicit enum tagging for durable JSON. Avoid ambiguous `untagged` enums
  unless variants are provably disjoint and tests cover ambiguity.
- Decide whether unknown fields are rejected, retained, or ignored based on the
  protocol's compatibility policy; do not accept them accidentally.
- Distinguish missing, `null`, empty, and default values deliberately.
- Use checked integer conversions and define overflow behavior.
- Canonical hashing/signing inputs require deterministic field, map, set, float,
  Unicode, and timestamp rules.
- Never sign one representation and verify a semantically “equivalent” but
  differently canonicalized representation.
- Golden fixtures MUST include prior supported versions and malformed/adversarial
  values.

### SQL, paths, commands, and URLs

- Use parameterized SQL for values. Centralize and test identifier quoting for
  dialects that cannot bind identifiers.
- Do not assemble SQL from domain strings through scattered `format!` calls.
- Treat filesystem paths as `Path`/`PathBuf`, not UTF-8 strings. Normalize and
  constrain paths at the boundary where traversal matters.
- Never invoke a shell for an operation expressible with `Command` arguments or
  a library API. Do not concatenate untrusted command text.
- Parse URLs into a URL type before policy decisions; string prefixes are not
  origin or endpoint validation.

### Resource limits

Safe Rust can still be denied service. Bound:

- request and document size;
- recursion and nesting depth;
- collection counts;
- decompressed output;
- graph nodes/edges and query work;
- concurrent tasks and queued messages;
- retries, redirects, and response reads;
- diagnostic/error payload size.

Reject oversized input with a typed boundary error before expensive parsing or
allocation where practical.

### Secrets and evidence

- Use redacted `Debug`/`Display` implementations for secret-bearing types.
- Do not derive `Debug` blindly on credentials, tokens, private prompts, raw
  governed rows, or signing material.
- Zeroization is necessary only for threat models where memory remnants matter;
  use a vetted crate rather than a hand-written volatile loop.
- Bind evidence to subject, audience/resource, operation, policy version,
  catalog identity, timestamp/expiry, and canonical payload as required by the
  protocol.
- Compare and classify cryptographic failures through vetted APIs, not custom
  cryptography.

## Tests live in separate files

All Rust test bodies MUST live in files separate from production source. Do not
add an inline `#[cfg(test)] mod tests { ... }` block. Existing inline test bodies
are migration debt: when materially changing such a module, extract its tests
as part of the change unless doing so would make the task unsafe or unreviewable.

### Unit-test file layout

Keep the test module a child of the production module so it can exercise private
invariants without widening production visibility.

For a flat source file:

```text
src/validation.rs
src/validation_tests.rs
```

At the end of `src/validation.rs`, declare only the module:

```rust
#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
```

The `path` is relative to the directory containing `validation.rs`, as specified
by the [Rust Reference's module-path
rules](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute).

`src/validation_tests.rs` contains the test code:

```rust
use super::*;

#[test]
fn rejects_a_manifest_with_a_mismatched_digest() {
    // Arrange, act, and assert the observable contract.
}
```

For a directory-backed module, use the existing natural layout:

```text
src/lakecat/mod.rs
src/lakecat/tests.rs
```

with:

```rust
#[cfg(test)]
mod tests;
```

For a nested flat module such as `src/agent/interop.rs`, either use a sibling
file with an explicit path:

```rust
#[cfg(test)]
#[path = "interop_tests.rs"]
mod tests;
```

which loads `src/agent/interop_tests.rs`, or use
`src/agent/interop/tests.rs` with plain `mod tests;`. Follow the nearest module's
existing separate-file convention and choose the layout that keeps related
files easy to find.

The `#[cfg(test)]` declaration may remain in production source; no test function,
fixture, fake, or test-only implementation may.

### Integration-test layout

Use `tests/` for behavior through the public crate API, executable/transport
behavior, persisted fixtures, and cross-module integration. Cargo builds every
top-level integration test file as a separate crate/binary, so prefer one or a
small number of harness roots split into modules:

```text
tests/querygraph.rs
tests/querygraph/server.rs
tests/querygraph/bundles.rs
tests/support/mod.rs
```

The harness root declares nested modules explicitly so the files below
`tests/querygraph/` remain modules of one integration-test crate rather than
independent test targets:

```rust
#[path = "querygraph/bundles.rs"]
mod bundles;
#[path = "querygraph/server.rs"]
mod server;
#[path = "support/mod.rs"]
mod support;
```

Do not create dozens of top-level integration test crates. The Cargo Book
documents the compile/link cost, and Kladov's
[test-layout guidance](https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html)
explains why modular unit/integration harnesses improve iteration time.

Do not make internal APIs public solely for an integration test. Use a child
unit test, test a real public contract, or reconsider the module boundary.

### Test taxonomy

Use the cheapest test that proves the property:

- Unit tests: pure transformations, constructors, exact error variants,
  boundary conditions, and private invariants.
- Table-driven tests: many examples with identical assertion structure.
- Property tests: algebraic laws, round trips, canonicalization, parsers,
  arbitrary graph/data shapes, and state-machine invariants.
- Compile-fail tests (for example `trybuild`): forbidden typestate transitions,
  witness forgery, trait-bound/API diagnostics, and macro misuse.
- Integration tests: public API composition, storage, server routes, adapter
  contracts, and process behavior.
- Golden/snapshot tests: stable external representations where a reviewed diff
  is meaningful; never as a substitute for semantic assertions.
- Fuzz tests: parsers, deserializers, canonicalizers, graph loaders, and unsafe
  boundaries.
- Loom tests: custom synchronization and CAS state machines.
- Benchmarks: performance distributions, never correctness substitutes.

### Test quality rules

- Name tests after behavior and conditions, not implementation method names.
- Assert the strongest stable semantic result: exact enum variant and key fields,
  not just `is_err` or a full brittle debug string.
- Cover happy path, each failure category, boundaries, empty/single/many values,
  overflow/size limits, and security non-disclosure.
- Keep tests deterministic. Inject clocks, IDs, randomness, and external
  services. Do not depend on wall-clock sleeps, hash iteration order, local
  timezone, ambient environment, or live internet unless explicitly marked as
  an external test.
- Use temporary directories and ephemeral ports; never a developer's real data
  path.
- A regression test must fail for the buggy implementation for the intended
  reason.
- Do not duplicate production algorithms in assertions. Use an independent
  oracle, a simple reference implementation, a law, or a fixture with reviewed
  expected output.
- Keep one conceptual reason for failure per test.
- Avoid tests that only mirror struct construction and then assert every field
  equals its input unless construction enforces a meaningful contract.

### Test DRYness

Extract stable fixtures, builders, fake capabilities, and domain assertions into
`test_support` modules when several tests share semantics. Keep defaults valid
and make the unusual condition visible at each call site.

Do not create a giant fixture that initializes every subsystem for every test.
Provide focused builders and capabilities. Repetition in a test may be clearer
than a helper with boolean flags, positional arguments, or hidden assertions.
DRY the domain setup, not the behavior being specified.

### Fuzzing and deeper verification

Add fuzz targets for untrusted bytes and complex parsers. Seed with valid and
historical fixtures so mutations reach deep logic. Promote every minimized crash
to a deterministic regression test.

Use:

- the [Rust Fuzz Book](https://rust-fuzz.github.io/book/) for fuzzing;
- [Proptest](https://proptest-rs.github.io/proptest/proptest/index.html) for
  generated values and shrinking;
- [Miri](https://github.com/rust-lang/miri) for undefined-behavior detection;
- [Loom](https://docs.rs/loom/latest/loom/) for concurrent interleavings;
- [Kani](https://model-checking.github.io/kani/) selectively for small,
  high-value bounded proofs.

These tools complement one another. Passing examples, Miri, or a fuzzer does
not prove soundness or general correctness.

### Documentation examples

Public rustdoc examples are documentation and MAY remain with the documented
item, but keep them short and focused on usage. All substantial behavioral test
logic belongs in separate test files. Doctests do not replace unit and
integration tests.

## Documentation, comments, and naming

Readable Rust makes domain structure evident from types and names, then uses
documentation to explain purpose, invariants, and tradeoffs.

### Rustdoc

Document every intentional public item. Public documentation should contain, as
applicable:

- one-sentence purpose;
- domain semantics and invariants;
- ownership or lifecycle behavior that is not obvious;
- `# Errors` with semantic failure categories;
- `# Panics` for reachable panic conditions;
- `# Safety` for every unsafe contract;
- a minimal useful example;
- compatibility or canonicalization guarantees.

Do not restate parameter and return types in prose. Explain what the signature
cannot: why, units, ordering, identity, side effects, complexity, and guarantees.
Follow the
[rustdoc writing guide](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html).

Use module-level `//!` documentation to state a module's responsibility,
dependency direction, and invariants. For advanced type-level code, include a
plain-language state-machine description before implementation details.

### Comments

Comments explain:

- why an approach is correct;
- why a tempting alternative is wrong;
- a protocol or security invariant;
- a performance result and benchmark;
- a non-obvious lifetime/ownership relationship;
- compatibility behavior;
- a `SAFETY` proof.

Delete comments that narrate syntax, repeat a name, or become false after a
refactor. Keep issue links with enough local explanation that the code remains
understandable if the link disappears.

### Naming

- Use domain vocabulary consistently across modules and languages.
- Name types as nouns, operations as verbs, predicates as `is_`/`has_`/`can_`
  when that reads naturally, and conversions by Rust convention.
- Include units in names or types: `timeout_ms`, `size_bytes`, `RowCount`.
- Avoid generic names (`data`, `info`, `manager`, `handler`, `processor`,
  `util`) when a domain term exists.
- Avoid abbreviations except established project/protocol vocabulary.
- Avoid encoding the backing representation in a domain name.
- Error variants describe the condition, not the place that noticed it.
- A trait name states a capability or behavior; do not append `Trait`.

Use `rustfmt` as the formatting authority and default Rust style. Do not hand
align fields or fight the formatter. The
[Rust Style Guide](https://doc.rust-lang.org/style-guide/) explains the
readability rationale.

### Function size and control flow

There is no mechanical maximum line count. Extract when a function mixes
levels of abstraction, owns several independent effects, or cannot be named as
one operation.

Keep orchestration control flow high and work routines straight-line. Kladov's
[“Push Ifs Up And Fors Down”](https://matklad.github.io/2023/11/15/push-ifs-up-and-fors-down.html)
is a useful heuristic: callers choose whether work happens; focused callees
receive valid preconditions and perform the work. Do not apply it blindly when
it would duplicate security checks—the validated type should instead carry the
precondition.

Prefer early `return`, `?`, and `let ... else` to deep nesting when the early
path is exceptional. Prefer a top-level exhaustive match when alternatives are
the domain.

## Dependencies, features, and toolchains

Every dependency is build time, supply-chain surface, MSRV pressure, binary
weight, and API commitment.

### Adding a dependency

Before adding one:

- confirm `std` and existing dependencies do not already solve the problem;
- inspect maintenance, release cadence, ownership, license, security history,
  unsafe usage, transitive graph, default features, MSRV, and platform support;
- choose minimal features and disable heavyweight defaults when appropriate;
- prefer a focused established crate over a broad framework for one utility;
- record why it is needed in the change/commit, not a permanent comment unless
  the choice is non-obvious;
- run `cargo tree` and check duplicate major versions.

Do not add a crate merely to avoid ten lines of clear stable code. Do not
reimplement cryptography, URL parsing, Unicode, concurrency primitives, or a
complex standard when vetted crates exist.

### Features

- Features should be additive capabilities, not mutually exclusive global
  modes when avoidable.
- Do not let enabling a feature silently remove security or validation.
- Keep feature names semantic and stable.
- Gate optional dependencies with the feature that needs them.
- Test supported combinations and document invalid ones.
- Avoid `cfg` branches that duplicate substantial domain logic; abstract the
  adapter and share the core.

### Stable Rust and MSRV

Use stable Rust unless a task explicitly authorizes nightly. Diagnostic use of
nightly tools (`-Zprint-type-sizes`, Miri) must not make production depend on
nightly.

The root crate currently uses Rust edition 2024. Preserve that edition unless a
deliberate migration says otherwise. An edition selects language behavior; it
does not declare the minimum compiler version.

Declare `rust-version` when QueryGraph has an intentional minimum supported Rust
version, and test it in CI. Do not infer MSRV only from the edition. Raise MSRV
as an explicit compatibility decision with release notes. Cargo documents the
contract in its
[`rust-version` guide](https://doc.rust-lang.org/cargo/reference/rust-version.html).

Commit `Cargo.lock` for this application crate and use `--locked` for release
and final benchmark builds. Update dependencies intentionally and review lockfile
diffs.

### Lints and formatting

- CI warnings are errors; keep `cargo clippy --all-targets -- -D warnings`
  clean.
- Do not enable the entire Clippy `restriction` group; its lints intentionally
  conflict. Select justified lints individually.
- A lint suppression must be narrow and include `reason = "..."` where the
  compiler supports it.
- Fix the design instead of globally allowing `too_many_arguments`,
  `large_enum_variant`, or `type_complexity` unless the exception is genuinely
  clearer or measured.
- Never add `#![allow(warnings)]` or broad warning suppression.
- Run rustfmt; do not manually reformat unrelated files.

### Build-time quality is quality

Avoid gratuitous proc macros, huge generated impl matrices, deeply nested
generic types, and generic large functions. Keep frequently changed modules
from depending on large adapter stacks when a boundary can isolate them.
Measure compile time and `cargo llvm-lines` when an abstraction materially
expands monomorphization. Runtime speed does not excuse a development loop that
becomes prohibitively slow when a concrete/dynamic cold path performs equally.

## Patterns to reject

Reject these during review unless a documented exception applies:

- Stringly typed domain states, identities, actions, errors, or versions.
- Boolean/`Option` soup representing mutually exclusive states.
- Public invariant-bearing fields.
- Repeated validation after a smart constructor.
- `serde_json::Value` propagated through known domain logic.
- A giant context/state/service object passed everywhere.
- `Arc<Mutex<_>>` as the first ownership design.
- Lock guards held across `.await` without explicit necessity.
- Unbounded tasks, channels, retries, fan-out, or input.
- Fire-and-forget writes or commits.
- `clone` used blindly to silence borrow errors.
- `unsafe` used to silence bounds or borrow errors.
- `unwrap`/`expect` on environmental or attacker-controlled outcomes.
- `panic!` for ordinary input or adapter failure.
- Wildcard matches on internal domain enums.
- Error categories erased into strings.
- Logging the same error at every propagation layer.
- Traits with one implementation and no real boundary.
- Generic parameters that do not change behavior or representation.
- GAT/HRTB/typestate designs whose callers immediately erase the types.
- `Deref` used for inheritance or domain-newtype convenience.
- Custom `Into`-style traits when standard conversions are exact.
- Macro DSLs hiding ordinary Rust control flow.
- A `utils` module accumulating unrelated helpers.
- Premature caching, interning, arenas, small-vector/string types, custom
  hashers, or custom allocators.
- `#[inline(always)]` or representation attributes without evidence/contract.
- Performance claims from debug builds or single noisy samples.
- Duplicated fast/slow paths without shared conformance tests.
- Inline test bodies in production source.
- Integration-test files multiplied into many separately linked crates.
- Tests that assert only `is_ok`/`is_err` when variants matter.
- Test fixtures that hide the condition under test.
- Broad Clippy allows or a warning downgrade.
- Comments that narrate the code rather than preserve reasoning.
- Clever one-liners that obscure ownership, effects, or errors.

## Canonical QueryGraph patterns

These examples illustrate the target style. Adapt names and details to the real
domain; do not copy them mechanically.

### Typed boundary into a pure core

```rust
#[derive(Debug, serde::Deserialize)]
struct ImportRequestDto {
    bundle: serde_json::Value,
    catalog: String,
}

#[derive(Debug)]
struct ImportRequest {
    bundle: UnverifiedBundle,
    catalog: CatalogIdentity,
}

impl TryFrom<ImportRequestDto> for ImportRequest {
    type Error = InvalidImportRequest;

    fn try_from(dto: ImportRequestDto) -> Result<Self, Self::Error> {
        Ok(Self {
            bundle: UnverifiedBundle::parse(dto.bundle)?,
            catalog: CatalogIdentity::parse(dto.catalog)?,
        })
    }
}

fn plan_import(
    bundle: &VerifiedBundle,
    authority: &CatalogGrant,
) -> Result<ImportPlan, PlanImportError> {
    // Deterministic, typed planning with no I/O.
    ImportPlan::for_verified_bundle(bundle, authority)
}
```

The transport owns DTO parsing; verification mints `VerifiedBundle`; planning
cannot accidentally accept raw JSON.

### Outcome ADT and transport mapping

```rust
enum RecallOutcome {
    Found(AuthorizedMemory),
    NotFound,
    Denied(PolicyDenial),
}

fn recall(
    request: RecallRequest,
    grant: &VerifiedGrant,
) -> Result<RecallOutcome, RecallServiceError> {
    // Domain/application behavior.
    recall_authorized(request, grant)
}

fn recall_response(outcome: RecallOutcome) -> Response {
    match outcome {
        RecallOutcome::Found(memory) => ok(memory),
        RecallOutcome::NotFound => not_found(),
        RecallOutcome::Denied(denial) => forbidden(redact(denial)),
    }
}
```

Service outage remains the `Err` channel; domain outcomes remain an exhaustive
sum type; HTTP mapping stays at the transport edge.

### Closed command set for an owning task

```rust
enum CatalogCommand {
    Read {
        key: TableKey,
        reply: tokio::sync::oneshot::Sender<Result<Table, CatalogError>>,
    },
    Commit {
        application: AuthorizedApplication,
        reply: tokio::sync::oneshot::Sender<Result<Receipt, CatalogError>>,
    },
    Shutdown,
}
```

The enum is a composable protocol. A bounded `mpsc` channel adds backpressure;
one task owns mutable connection state; each request has an observed response.

### Thin generic facade, concrete implementation

```rust
pub fn verify_file(
    path: impl AsRef<Path>,
) -> Result<Verification, VerifyError> {
    verify_path(path.as_ref())
}

fn verify_path(path: &Path) -> Result<Verification, VerifyError> {
    let bytes = std::fs::read(path)
        .map_err(|source| VerifyError::Read {
            path: path.to_owned(),
            source,
        })?;
    verify_bytes(&bytes)
}

fn verify_bytes(bytes: &[u8]) -> Result<Verification, VerifyError> {
    // Reusable pure parsing and verification core.
    Verification::parse(bytes)
}
```

The large bodies are concrete, diagnostics remain typed, and tests can exercise
`verify_bytes` without filesystem setup.

### Performance-specific alternate implementation

```rust
fn canonical_digest(nodes: &[Node]) -> Digest {
    canonical_digest_single_pass(nodes)
}

// This duplicates the reference implementation's traversal because the
// benchmark in benches/catalog_digest.rs shows a material allocation and
// latency reduction for production graph sizes. Keep both implementations
// aligned through canonical_digest_matches_reference in the separate test file.
fn canonical_digest_single_pass(nodes: &[Node]) -> Digest {
    // Measured optimized implementation.
    digest_nodes_single_pass(nodes)
}

#[cfg(test)]
fn canonical_digest_reference(nodes: &[Node]) -> Digest {
    // Obviously correct, simple oracle available only to tests.
    digest_nodes_reference(nodes)
}
```

Use this pattern rarely. Real code should cite actual measurements, not the
placeholder prose above.

## Review and completion checklists

### Representation

- [ ] Do types represent only valid combinations where practical?
- [ ] Are alternatives an enum and conjunctions a cohesive struct?
- [ ] Are raw, validated, verified, and authorized values distinct?
- [ ] Are invariant-bearing fields private with one constructor/parser?
- [ ] Are booleans, strings, and `Option`s semantically appropriate?
- [ ] Are matches exhaustive where future variants require review?

### Functional design and DRYness

- [ ] Is decision logic pure or explicitly parameterized by effects?
- [ ] Are mutations narrow and ownership transitions visible?
- [ ] Does each domain rule have one source of truth?
- [ ] Does every abstraction compress a named concept rather than token
      similarity?
- [ ] Would a concrete function or ADT be clearer than a trait/type-level form?
- [ ] Are iterator chains readable and free of hidden side effects?

### API and modularity

- [ ] Does the dependency direction point inward?
- [ ] Are adapter/wire types contained at boundaries?
- [ ] Is visibility minimal and the public surface intentional?
- [ ] Do conversions use standard traits with exact semantics?
- [ ] Are semver, persisted format, and auto-trait changes understood?
- [ ] Is advanced type machinery documented in plain language?

### Errors, security, and resources

- [ ] Are actionable failures distinct typed variants?
- [ ] Are sources preserved without duplicate display text?
- [ ] Are panic/unwrap/expect uses true local invariants?
- [ ] Do authorization and verification fail closed without leaking data?
- [ ] Are input, queue, retry, task, and memory bounds explicit?
- [ ] Are secrets and backend-controlled messages safely handled?

### Async/concurrency review

- [ ] Is async used only for actual waiting?
- [ ] Is blocking/CPU work off the runtime worker pool?
- [ ] Do no ordinary lock guards survive `.await`?
- [ ] Are channels and in-flight operations bounded?
- [ ] Does every spawned task have ownership, result observation, and shutdown?
- [ ] Are cancellation, retries, idempotency, and commit boundaries defined?
- [ ] Is any atomic ordering supported by a written argument and Loom tests?

### Performance

- [ ] Is there a production-representative optimized baseline?
- [ ] Does profiling identify the changed bottleneck?
- [ ] Are algorithm, round trips, and repeated work addressed before micro-tuning?
- [ ] Are throughput, tails, errors/conflicts, CPU, and memory measured where
      relevant?
- [ ] Does the gain exceed noise and justify complexity?
- [ ] Are generic/dispatch, allocation, type-size, and compile-time costs known?
- [ ] Is a performance exception documented and guarded by benchmarks/tests?

### Tests and documentation

- [ ] Are all test bodies and test support in separate files?
- [ ] Does a regression test fail against the old bug?
- [ ] Are exact variants, boundaries, laws, and negative cases covered?
- [ ] Are clocks, IDs, randomness, order, environment, and services controlled?
- [ ] Does public rustdoc explain purpose, invariants, errors, panics, and safety?
- [ ] Do comments preserve reasoning instead of narrating syntax?

### Final gates

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all-targets`
- [ ] Public docs build warning-free when relevant.
- [ ] Optimized locked build and benchmark pass when performance changes.
- [ ] Miri/fuzz/property/Loom/compile-fail checks run when applicable.
- [ ] Final diff and `git status` contain only intended changes.

## Intellectual lineage and primary sources

This guide synthesizes primary Rust Project documentation and writing by people
who designed Rust, led its language/library/compiler teams, built foundational
Rust systems, or developed authoritative work in a specialty. There is no
canonical finite set of “all top contributors,” and not every major contributor
publishes a language-design blog. The corpus below is therefore selected by
demonstrated responsibility and relevance, not popularity. Historical posts are
used for enduring design reasoning; current stable official documentation wins
if syntax or semantics have changed.

### Rust Project sources

| Source | Guidance distilled here |
| --- | --- |
| [The Rust Programming Language](https://doc.rust-lang.org/stable/book/) | Ownership, enums/pattern matching, traits, errors, tests, closures, and iterators as the idiomatic base. |
| [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) | Naming, common traits, newtypes, validation, object safety, documentation, future-proofing, and meaningful errors. |
| [Rust Reference](https://doc.rust-lang.org/reference/) | The actual stable language guarantees; especially layout, modules, traits, and undefined behavior. |
| [Rustonomicon](https://doc.rust-lang.org/nomicon/) | Unsafe invariants, `PhantomData`, variance, drop checking, layout, ownership, and FFI. |
| [Rust Style Guide](https://doc.rust-lang.org/style-guide/) | Consistent rustfmt-led presentation reduces cognitive load. |
| [rustdoc book](https://doc.rust-lang.org/rustdoc/) | Purpose-led docs, examples, and explicit Errors/Panics/Safety contracts. |
| [Cargo Book](https://doc.rust-lang.org/cargo/) | Targets, separate integration crates, profiles, features, dependency resolution, and MSRV. |
| [Clippy lint index](https://rust-lang.github.io/rust-clippy/stable/) | Machine-checkable correctness, idiom, complexity, and performance review; restriction lints selected individually. |
| [Inside Rust Error Handling Project Group](https://blog.rust-lang.org/inside-rust/2021/07/01/What-the-error-handling-project-group-is-working-towards/) | Preserve source chains, separate layer context, and never duplicate a source in both `Display` and `source`. |
| [Rust Fuzz Book](https://rust-fuzz.github.io/book/) and [Miri](https://github.com/rust-lang/miri) | Exercise untrusted inputs and unsafe behavior while recognizing testing is not a proof. |

### Language and library design leaders

| Contributor / source | Role in the corpus | Guidance distilled here |
| --- | --- | --- |
| Graydon Hoare, Rust's original designer — [10 Years of Stable Rust](https://rustfoundation.org/media/10-years-of-stable-rust-an-infrastructure-story/) | Original language design and historical perspective. | Good systems-language ideas succeed through balanced, practical, industrially usable design—not maximal theory alone. |
| Niko Matsakis — [Being Rusty: design axioms](https://smallcultfollowing.com/babysteps/blog/2023/12/07/rust-design-axioms/) and [Rust/Python/TypeScript](https://smallcultfollowing.com/babysteps/blog/2025/07/31/rs-py-ts-trifecta/) | Long-time language-team leader, trait/borrow-checker architect. | Surface bugs early; make meaning transparent; keep advanced types accessible; enums encode domain state and guide context-limited humans and agents. |
| Aaron Turon — [Abstraction without overhead](https://blog.rust-lang.org/2015/05/11/traits/), [zero-cost futures](https://aturon.github.io/blog/2016/08/11/futures/), and [archive](https://aturon.github.io/blog/archive/) | Former core/library leader and major traits, futures, and API designer. | Traits and ownership can deliver composable abstraction near hand-written performance; design the abstraction and benchmark together. |
| withoutboats — [Zero Cost Abstractions](https://without.boats/blog/zero-cost-abstractions/) and [blog](https://without.boats/blog/) | Language/async designer. | “Zero cost” includes no global tax and excellent generated code, but an unusable abstraction still fails; ergonomics and explicit allocation matter. |
| Huon Wilson — [Finding Closure in Rust](https://huonw.github.io/blog/2015/05/finding-closure-in-rust/) and [object safety](https://huonw.github.io/blog/2015/05/where-self-meets-sized-revisiting-object-safety/) | Early language/library contributor. | Closures, iterators, static dispatch, and trait objects are composable choices; design traits intentionally for sized/generic and dynamic use. |
| Manish Goregaokar — [Rust archive](https://manishearth.github.io/blog/categories/rust/), [sum/product types](https://manishearth.github.io/blog/2017/03/04/what-are-sum-product-and-pi-types/), and [shared mutability](https://manishearth.github.io/blog/2015/05/17/the-problem-with-shared-mutability/) | Rust/Servo contributor and type-system educator. | Use ADTs to control state-space cardinality; aliasing plus mutation complicates invariants; compose wrappers by their exact guarantee and cost. |
| Will Crichton — [Type-Driven API Design](https://willcrichton.net/rust-api-type-patterns/introduction.html) and [type-level programming](https://willcrichton.net/notes/type-level-programming/) | Rust ownership/type-system researcher and educator. | Replace stringly/dynamic agreements with enums, witnesses, guards, and selective typestate; types should enforce consistency between related API elements. |
| Cliff Biffle — [The Typestate Pattern in Rust](https://cliffle.com/blog/rust-typestate/) | Systems practitioner with a canonical practical typestate treatment. | Consuming transitions and state-specific methods can eliminate illegal operation order and runtime checks; state data can live in state types. |
| Aleksey Kladov (matklad) — [blog](https://matklad.github.io/), [ARCHITECTURE.md](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html), [Concrete Abstraction](https://matklad.github.io/2020/08/15/concrete-abstraction.html), [Fast Rust Builds](https://matklad.github.io/2021/09/04/fast-rust-builds.html), and [test layout](https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html) | Co-founder of rust-analyzer and large-Rust-codebase designer. | Preserve a code map and explicit boundaries; charge abstractions for cognitive/compile cost; keep generic facades thin; keep test bodies in separate files and integration roots few. |

### Performance, unsafe, and concurrency leaders

| Contributor / source | Role in the corpus | Guidance distilled here |
| --- | --- | --- |
| Nicholas Nethercote et al. — [Rust Performance Book](https://nnethercote.github.io/perf-book/) | rustc/Firefox performance expert and the principal practical Rust performance reference. | Profile first; optimize allocation, type size, hashing, iterators, inlining, and compile time based on measured hot paths. |
| Andrew Gallant (BurntSushi) — [blog](https://burntsushi.net/) and [ripgrep analysis](https://burntsushi.net/ripgrep/) | Author of ripgrep and Rust's regex ecosystem. | Benchmark realistic workloads rigorously; account for algorithms, Unicode correctness, I/O, parallelism, and reproducibility; put optimizations in reusable lower layers. |
| Ralf Jung — [blog](https://www.ralfj.de/blog/), [Scope of Unsafe](https://www.ralfj.de/blog/2016/01/09/the-scope-of-unsafe.html), and [pointer provenance](https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html) | Rust memory-model researcher and Miri maintainer. | Unsafe correctness rests on explicit invariants and provenance; privacy creates an auditable safe-abstraction boundary; Miri detects executions, not universal soundness. |
| Aria Beingessner (Gankra) — [Faultlore](https://faultlore.com/blah/) and [Rust, Lifetimes, and Collections](https://faultlore.com/blah/rust-lifetimes-and-collections/) | Standard-collections/unsafe contributor and original Nomicon author. | Isolate low-level unsafety behind safe iterators/collections; understand ZSTs, layout, drop/leak behavior, and `Send`/`Sync`; prefer safe zero-cost interfaces. |
| Mara Bos — [Rust Atomics and Locks](https://marabos.nl/atomics/) | Rust library-team leader and concurrency specialist. | Choose locks/atomics from a memory-model proof, not folklore; understand ordering and platform behavior before custom primitives. |
| Alice Ryhl — [blog](https://ryhl.io/blog/), [blocking in async](https://ryhl.io/blog/async-what-is-blocking/), and [actors](https://ryhl.io/blog/actors-with-tokio/) | Tokio maintainer. | Keep blocking work off async workers; prefer short lock scopes or an owning task; typed bounded messages isolate I/O and state complexity. |
| Tyler Mandry — [blog](https://tmandry.gitlab.io/blog/), [async optimization](https://tmandry.gitlab.io/blog/posts/optimizing-await-1/), and [async reliability](https://tmandry.gitlab.io/blog/posts/making-async-reliable/) | Rust language/async leader. | Async functions are enum-like state machines; values across awaits affect size; cancellation, starvation, and scoped task ownership are reliability concerns. |
| Carl Lerche / Tokio team — [Making the Tokio scheduler 10x faster](https://tokio.rs/blog/2019-10-scheduler) and [Tokio topics](https://tokio.rs/tokio/topics) | Foundational async runtime design and implementation. | Combine mechanical sympathy, profiling, and correctness tooling such as Loom; scheduler speed never excuses untested concurrency. |

### Supporting pattern references

- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) is a useful
  community catalog of idioms, patterns, and anti-patterns. Focus on why and
  tradeoffs, not pattern collection.
- [Type-Driven API Design in Rust](https://willcrichton.net/rust-api-type-patterns/)
  is the practical reference for witnesses, guards, typestate, and consistency.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) remain the
  baseline for ecosystem-facing API design.
- The latest stable [Reference](https://doc.rust-lang.org/reference/) and
  standard-library documentation override old blog syntax or implementation
  details.

The distilled conclusion is deliberately conservative and ambitious at once:
use Rust's ADTs, ownership, traits, closures, iterators, witnesses, and selective
type-level machinery aggressively against real invalid states; use profiling,
dynamic boundaries, concrete code, and plain control flow aggressively against
accidental complexity. Beautiful QueryGraph Rust makes the correct path easy,
the wrong path unrepresentable where practical, and the fast path measurable.

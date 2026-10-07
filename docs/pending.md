# Pending commands

Import `bevy_promptline::command::pending::PendingAppExt` to register a
`ConsoleCommand` handler that returns `CommandExecution`. Existing
`PromptlineAppExt` implementations and synchronous handler signatures remain
unchanged.

`CommandExecution::Finished` carries an existing `CommandOutcome`.
`CommandExecution::Pending` transfers a `PendingCommand` to the runtime.
`PendingCommand::channel()` creates that receiving endpoint and its separate
`CommandCompletion`. The factory does not start a thread or task. The handler
must start host-owned work and return without retaining its World borrow.

`CommandCompletion::finish(self, outcome)` consumes the worker endpoint.
A second finish cannot compile. The endpoint exposes no underlying Sender or
Receiver. Each channel can buffer one terminal outcome through this public
API. [Rust's channel documentation](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html)
controls channel delivery; the wrapper's consuming ownership bounds its sends.

The runtime polls after dispatch and before output consumers. It uses
[the standard nonblocking receive operation](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.try_recv)
and emits one terminal notification after receiving a result. Dropping an
unfinished worker endpoint produces `Failure("pending command completion disconnected")`.
The two compile-fail doctests cover completion consumption and read-only
completion-provider parameters.

`PendingLimits` defaults to 64 retained invocations. Its positive limit is
independent from the per-frame submission limit. When a handler returns a
pending endpoint beyond capacity, the handler has already run. The runtime
requests cancellation, drops the receiver, and emits
`Failure("pending command limit reached")`. It cannot undo earlier side effects.
A worker finishing after receiver removal gets `SendError<CommandOutcome>`.

Send `CancelCommand` with an invocation identity from this App's lifecycle
stream. Unknown and finished identities have no effect. The request sets the
worker's read-only `CommandCancellation` signal. The worker chooses its
terminal outcome; a request cannot force cancellation or undo completed work.
The cancellation token can outlive its completion endpoint.

Accepted metadata sensitivity stays attached to pending work. For redacted
commands, terminal failure notifications contain
`Failure("command failed for a sensitive command")` instead of worker text.
Public-command diagnostics retain their text. Success and cancellation retain
their existing outcome shapes. This policy covers runtime-owned lifecycle
messages; game-authored output and other log sinks retain their host policy.

Polling examines at most the configured retained count. Each cancellation
request searches that collection. Runtime storage retains receiving endpoints,
identities, origins, and sensitivity; it does not retain World access. Diagnostic
payload sizes remain caller-controlled, and the handle count is not a byte or
allocator-overhead guarantee.

The [kitchen-sink implementation](../examples/support/pending.rs) schedules
five seconds of game time through Bevy's Timer. `wait` starts work, `echo` remains
available while it runs, and `cancel` requests cancellation. The host observes
lifecycle messages to track running identities. Terminal task state has no
completion capability and the host despawns it after sending its outcome.

[Public tests](../tests/pending.rs) cover dispatch and analysis during stalled
work, exact terminal delivery, selective cancellation, worker acknowledgement,
worker failure, disconnection, capacity, removed limits, metadata rejection,
permission rejection, UI origin, diagnostic redaction, and opaque debug output.
The [compatibility fixture](../tests/app_ext_compatibility.rs) implements the
original synchronous extension trait without a pending method.

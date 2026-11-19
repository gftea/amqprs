# Design: publish that returns its confirm as a future (issue #201)

## Context
In confirm mode an ack or nack only reaches the user through `ChannelCallback::publish_ack` / `publish_nack`. To wait for one publish's confirm, users must count delivery tags and match them in the callback themselves. The issue asks for `basic_publish_confirmed(...) -> Result<PublishConfirm>`, where awaiting the returned future gives `Confirmation::Ack | Nack`. The existing callbacks keep firing as they do today.

The hard part is **which delivery tag a publish gets**. RabbitMQ numbers publishes 1, 2, 3… on each channel in the order they reach the wire, counting every publish after `confirm.select`, including plain `basic_publish`. The client has to give each publish the same number. Today, publishing works like this:
- `Channel::basic_publish` (`api/channel/basic.rs:928`) does `outgoing_tx.send((id, Frame::PublishCombo(..))).await`. The queue is a bounded mpsc shared by the whole connection.
- `WriterHandler` (`net/writer_handler.rs`) is the only reader of that queue, so it alone sees the true wire order.
- Acks and nacks come back through `ReaderHandler` to `ChannelDispatcher` (`api/channel/dispatcher.rs:545`). The dispatcher's `select!` is `biased`, so a management command queued earlier is always handled before a later incoming frame.
- `Channel` is `Clone`. The docs advise against sharing one channel between tasks, but nothing stops it.

## Options compared

| | A. Count tags in `Channel` (atomic counter) | B. Count tags in the writer task | C. Send confirm-mode publishes through the dispatcher |
|---|---|---|---|
| How | `SharedChannelInner` holds `AtomicU64 next_tag`. The publish takes the next tag, sends a `RegisterConfirm{tag, tx}` command to the dispatcher (unbounded, no await), then queues the frame. | The outgoing message carries an optional confirm sender. The writer keeps a counter for each channel in confirm mode, turned on when it writes `confirm.select` and reset on channel close. It registers the tag with that channel's dispatcher before writing the bytes. | The dispatcher assigns the tag and forwards the frame to `outgoing_tx` itself. |
| Correct with concurrent publishers on cloned channels | **No**, unless guarded (see A+ below). On a multi-thread runtime, two tasks can take tags in one order and queue frames in the other. Acks then resolve the wrong futures, with no error. | **Yes.** The writer is the single place where order is decided. | **Yes.** Tags are assigned in the per-channel dispatcher. |
| Caller cancels the publish (`timeout` or `select!`) | The naive version burns a tag when the future is dropped while `send().await` is waiting, so **every later confirm is off by one**. Fixed by `outgoing_tx.reserve().await` first, then tag, register and `permit.send()` with no await between them. | Safe. The tag is only assigned once the frame has been dequeued. | Safe once the frame is in the dispatcher's queue. |
| Layering | Stays in `api/channel`. | `net` has to know about api-level confirm types and dispatcher commands. `OutgoingMessage` changes type at about 25 send sites. | Stays in `api/channel`. |
| Hot-path cost | One atomic op plus one unbounded send, and only in confirm mode. | A per-channel map lookup in the writer for every frame, on the connection-wide hot path. | An extra task hop per publish. The dispatcher awaits the bounded `outgoing_tx`, so publish backpressure stalls ack and delivery handling on that channel. |
| Size of change | Small | Large | Medium. Behaviour changes for existing users. |
| Precedent | Java client `getNextPublishSeqNo`, which has the same documented "don't share channels" caveat | lapin (which uses internal locks for this) | none |

**A+ (A made fail-loud):** an `AtomicBool publishing` guard, taken with `compare_exchange` around the await-free section (take tag, register, `permit.send`). If another task holds it, return `Error::ChannelUseError("concurrent publish on a confirm-tracked channel")` and don't take a tag. This keeps tag order equal to queue order, so attribution is never silently wrong. It never waits or spins, so it is a try-flag rather than a lock. To avoid changing anything for current users, it only applies once `basic_publish_confirmed` has been used on that channel (`tracking: AtomicBool`).

### Recommendation: A+
- It matches the issue's proposal and the project's priorities: lock-free, minimal, fast, and changes stay inside `api/`.
- `reserve()` makes it cancel-safe, which matters given the recent cancellation fixes (#204).
- The guard turns the one remaining hazard, concurrent publishers on one channel (already discouraged by the `Channel` docs and by RabbitMQ for confirms), into an error instead of wrong confirms.
- Choose **B** instead if concurrent publishing on a shared channel must *just work*. That costs `net`-layer coupling and a wider diff.

## Implementation (A+)

**`api/channel/mod.rs`:** add to `SharedChannelInner`:
- `confirm_mode: AtomicBool`
- `next_tag: AtomicU64`
- `tracking: AtomicBool`
- `publishing: AtomicBool`

Add a `DispatcherManagementCommand::RegisterConfirm { tag, tx: oneshot::Sender<Confirmation> }`.

**`api/channel/confim.rs`:** in `confirm_select`, before sending `Select` (both the `no_wait` path and the normal one), set `next_tag = 1` and `confirm_mode = true`.

**`api/channel/basic.rs`:**
- Factor the frame building into a helper.
- `basic_publish`: if `confirm_mode`, use `reserve().await`. Then, if `tracking`, take the guard. Then `fetch_add`, then `permit.send`. Otherwise keep the current path unchanged.
- New `basic_publish_confirmed(props, content, args) -> Result<PublishConfirm>`:
  - Error if not in confirm mode.
  - Set `tracking`, `reserve().await`, take the guard (error if contended).
  - `fetch_add`, send `RegisterConfirm` (no await), `permit.send`, release the guard.
- New public types, re-exported via `pub use basic::*`:
  - `Confirmation { Ack(Option<Box<ReturnedMessage>>), Nack(Option<Box<ReturnedMessage>>) }`
  - `ReturnedMessage { ret: Return, basic_properties, content }`
  - `PublishConfirm`, which wraps `oneshot::Receiver<Confirmation>` and implements `Future<Output = Result<Confirmation>>`. A dropped sender (channel or connection closed) maps to `Error::ChannelUseError`.

**`api/channel/dispatcher.rs`:**
- Add `confirms: BTreeMap<u64, oneshot::Sender<Confirmation>>`, filled by `RegisterConfirm`.
- On `Ack`/`Nack`: if `multiple`, resolve every tag `<= delivery_tag` (using `split_off`); otherwise resolve that one tag. Then call the existing callback as today, and only if one is registered, so the existing "callback not registered" log stays.
- When the dispatcher exits, the map is dropped and pending futures resolve to an error.

**Returned `mandatory` messages (the issue's open question):** RabbitMQ sends `basic.return` before the `basic.ack` that covers the unroutable message.
- The dispatcher puts finished returns in a `VecDeque<ReturnedMessage>`, in addition to calling `publish_return` as now.
- On a single (non-multiple) ack, pop one buffered return and attach it to that tag's `Confirmation`.
- On a `multiple` ack the return can't be matched to a tag reliably. Attach nothing, log it with `traces`, and say in the docs that the callback stays the authoritative place to get returns.
- If the buffer has returns but no tracked confirm exists, discard them so they don't grow without bound.

**Docs and examples:**
- Rustdoc on `basic_publish_confirmed` covering the concurrency error, the multiple-ack caveat for returns, and cancel safety.
- New example `examples/src/publish_confirm_future.rs`, registered in `examples/Cargo.toml`.
- A README feature mention.

## Tests (`amqprs/tests/test_publish_confirm.rs`, using `tests/common`)
1. Publish N messages with `basic_publish_confirmed` without awaiting, then await all futures in order: all `Ack`.
2. Mix plain `basic_publish` and confirmed publishes: tags stay aligned and every future is `Ack`.
3. `mandatory` publish to an unroutable key: `Ack(Some(returned))`, and the callback also fires.
4. Calling it before `confirm_select` gives an error.
5. Cancel safety: wrap `basic_publish_confirmed` in a zero-duration timeout, then publish again: confirms are still correct.
6. Close the channel with confirms pending: the futures resolve to `Err`.
7. Two tasks on a cloned channel: never a wrong attribution. Each call either succeeds or returns `ChannelUseError`.

## Verification
- `./start_rabbitmq.sh`, then:
  - `cargo test --test test_publish_confirm`
  - `cargo test` for each of: `-F traces`, `-F compliance_assert`, `-F tls`, `-F urispec`
- `cargo clippy --all-features -- -Dwarnings`
- MSRV check: `cargo +1.71 build --all-features`
- `cargo run -p examples --example publish_confirm_future`
- If Docker isn't available in this container, run build, clippy and MSRV here and say plainly that the integration tests could not be run.

# Design: publish that returns its confirm as a future (issue #201)

Status: revision 2. It includes the fixes from the owner's review on PR 207.

## Context
In confirm mode an ack or nack only reaches the user through `ChannelCallback::publish_ack` / `publish_nack`. To wait for one publish's confirm, users must count delivery tags and match them in the callback themselves. The issue asks for `basic_publish_confirmed(...) -> Result<PublishConfirm>`, where awaiting the returned future gives `Confirmation::Ack | Nack`. The existing callbacks keep firing as they do today.

The hard part is **which delivery tag a publish gets**. RabbitMQ numbers publishes 1, 2, 3… on each channel in the order they reach the wire, counting every publish after the first `confirm.select`, including plain `basic_publish`. The client has to give each publish the same number. Today, publishing works like this:
- `Channel::basic_publish` (`api/channel/basic.rs`) does `outgoing_tx.send((id, Frame::PublishCombo(..))).await`. The queue is a bounded mpsc shared by the whole connection.
- `WriterHandler` (`net/writer_handler.rs`) is the only reader of that queue, so it alone sees the true wire order.
- Acks and nacks come back through `ReaderHandler` to `ChannelDispatcher` (`api/channel/dispatcher.rs`). The dispatcher's `select!` is `biased`, so a management command queued earlier is always handled before a later incoming frame.
- `Channel` is `Clone`. The docs advise against sharing one channel between tasks, but nothing stops it.

## Options compared

| | A. Count tags in `Channel` (atomic counter) | B. Count tags in the writer task | C. Send confirm-mode publishes through the dispatcher |
|---|---|---|---|
| How | `SharedChannelInner` holds `AtomicU64 next_tag`. The publish takes the next tag, sends a `RegisterConfirm{tag, tx}` command to the dispatcher (unbounded, no await), then queues the frame. | The outgoing message carries an optional confirm sender. The writer keeps a counter for each channel in confirm mode and registers each tag with the dispatcher before writing the bytes. | The dispatcher assigns the tag and forwards the frame to `outgoing_tx` itself. |
| Correct with concurrent publishers on cloned channels | **No**, unless guarded (see A+). Two tasks can take tags in one order and queue frames in the other, so acks resolve the wrong futures with no error. | **Yes.** The writer is the single place where order is decided. | **Yes.** Tags are assigned in the per-channel dispatcher. |
| Caller cancels the publish (`timeout` or `select!`) | The naive version burns a tag when the future is dropped mid-`send().await`, so every later confirm is off by one. Fixed by `reserve().await` first, then tag, register and `permit.send()` with no await between them. | Safe. | Safe once the frame is in the dispatcher's queue. |
| Layering | Stays in `api/channel`. | `net` has to know about api-level confirm types. `OutgoingMessage` changes type at about 25 send sites. | Stays in `api/channel`. |
| Hot-path cost | One atomic op plus one unbounded send, only in confirm mode. | A per-channel map lookup in the writer for every frame, on the connection-wide hot path. | An extra task hop per publish. Publish backpressure stalls ack and delivery handling on that channel. |
| Size of change | Small | Large | Medium |

## Decision: A+
Count tags in the channel, with three safeguards:
1. **Cancel safety:** `outgoing_tx.reserve().await` comes before taking a tag, so a dropped future never uses one up.
2. **A guard that fails loudly.** Every publish on a confirm-mode channel, plain or confirmed, takes an `AtomicBool publishing` flag with `compare_exchange` around a section with no awaits in it: take the tag, register it, `permit.send`.
   - If the flag is already held, the publish returns `Error::ChannelUseError` and nothing is sent or counted. Tag order therefore always equals queue order, and a confirm can never resolve the wrong future without an error.
   - It never waits or spins. An RAII guard releases the flag, so a panic can't leave it set.
3. **API contract: one publisher per channel in confirm mode.** This goes in the rustdoc for `basic_publish`, `basic_publish_confirmed`, `confirm_select` and `Channel`.
   - **Behaviour change:** existing code that publishes from several tasks on one confirm-mode channel can now get this error when those publishes overlap. It goes in the CHANGELOG.

B stays the fallback if publishing from several tasks on one channel ever has to just work.

## Implementation

**`api/channel/mod.rs`:**
- `SharedChannelInner` gains `confirm_mode: AtomicBool`, `next_tag: AtomicU64` and `publishing: AtomicBool`.
- New `DispatcherManagementCommand::RegisterConfirm { tag, tx: oneshot::Sender<Confirmation> }`.

**`api/channel/confim.rs`:** `confirm_select` does `confirm_mode.compare_exchange(false, true)` and sets `next_tag = 1` **only on that first switch**, before sending `Select`. RabbitMQ treats a repeated `confirm.select` as a no-op and keeps counting, so the counter must not reset.

**`api/channel/basic.rs`:**
- Factor the frame building into a helper.
- `basic_publish`, outside confirm mode: unchanged.
- `basic_publish`, in confirm mode: `reserve().await`, take the guard (error if contended), `fetch_add`, `permit.send`.
- New `basic_publish_confirmed(props, content, args) -> Result<PublishConfirm>`:
  - Error if the channel isn't in confirm mode.
  - `reserve().await`, then take the guard (error if contended).
  - `fetch_add`, then send `RegisterConfirm`. If the dispatcher is gone, return an error and do not send.
  - `permit.send`.
- New public types:
  - `#[non_exhaustive] enum Confirmation { Ack(Option<Box<ReturnedMessage>>), Nack }`. RabbitMQ answers an unroutable mandatory message with return + ack, never return + nack, so `Nack` carries nothing.
  - `ReturnedMessage { ret: Return, basic_properties, content }`. The internal `ReturnMessage` buffer is renamed `ReturnBuffer` to avoid confusion.
  - `PublishConfirm`: wraps `oneshot::Receiver<Confirmation>` and implements `Future<Output = Result<Confirmation>>`. A dropped sender (channel or connection closed) maps to an error.

**`api/channel/dispatcher.rs`:**
- `confirms: BTreeMap<u64, oneshot::Sender<Confirmation>>`, filled by `RegisterConfirm`.
- On `Ack`/`Nack`: if `multiple`, resolve every tag `<= delivery_tag` using `split_off`; otherwise resolve that one tag. Acks can arrive out of order across queues, and the map handles that. Then call the existing callback as today.
- On exit, the map is dropped and pending futures resolve to an error.

**Returned `mandatory` messages:**
- RabbitMQ sends `basic.return` and then the ack for an unroutable message while handling that one publish. So a return is always followed by the ack that covers its message.
- The dispatcher keeps **one `Option<ReturnedMessage>` slot**, filled when a complete return arrives. `publish_return` still fires as now.
- **On every ack or nack, take the slot.** On a single ack, attach it to that tag's `Confirmation::Ack`. On a multiple ack or a nack, drop it. Then a leftover return can never be attached to a later, different publish.
- On a multiple ack the return can't be matched to a tag, so `publish_return` stays the authoritative place to get returns. The rustdoc says so.

## Tests
- **Integration tests** (`amqprs/tests/test_publish_confirm.rs`):
  1. Publish N confirmed messages without awaiting, then await them in order: all `Ack(None)`.
  2. Mix plain and confirmed publishes: tags stay aligned.
  3. A mandatory, unroutable publish gives `Ack(Some(returned))` with the right body, and the callback also fires.
  4. `basic_publish_confirmed` before `confirm_select` gives an error.
  5. Calling `confirm_select` twice: tags stay aligned.
  6. Close the channel with confirms pending: the futures resolve to `Err`.
  7. Two tasks on a cloned channel, publishing mandatory, unroutable messages with unique bodies:
     - every successful future's attached return has its own body;
     - every failure is `ChannelUseError`.
- **Dispatcher and tag-logic unit tests** (synthetic frames or a small, full `mpsc`):
  - Cancel safety: drop the future while `reserve()` is waiting on a full queue, then publish again. The tags stay aligned.
  - A return, then a multiple ack, then a single ack for another tag: the single ack gives `Ack(None)`.
  - An out-of-order multiple ack resolves exactly the tags `<= delivery_tag` that are still pending.

## Verification
- `./start_rabbitmq.sh`, then `cargo test` with each of the feature flags (`traces`, `compliance_assert`, `tls`, `urispec`) and with `--all-features`.
- `cargo clippy --all-features -- -Dwarnings`.
- MSRV check: `cargo +1.71 build --all-features`.
- New example `examples/src/publish_confirm_future.rs`.

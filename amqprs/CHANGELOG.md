# Changelog

## Unreleased

- Keep the connection reader running until the close exchange completes. Incoming
  heartbeats and crossed Close requests no longer end a local close early.
- Continue a started close handshake when its caller is cancelled. The handshake
  runs on the runtime that opened the connection. If a handshake is still needed, closing
  after that runtime has stopped returns an error.
- Stop the reader and report an I/O failure when the writer exits, even if the
  peer keeps sending data.
- Ignore an abandoned responder-registration acknowledgement instead of panicking
  the reader.
- Once local close starts, discard ordinary channel frames and Blocked/Unblocked
  notifications. A callback already running still needs to return.

These changes do not add a close timeout or guarantee that every internal task
has joined before `close()` returns.

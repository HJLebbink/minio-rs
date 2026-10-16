# A retried HEAD request sends its headers twice

Status: described, not fixed. Line numbers are for `6db367b`.

## Summary

When `execute` retries a request after a `RetryHead` error, it passes the same
`Multimap` of headers to the second attempt. The second attempt adds its headers
to the ones the first attempt left behind. The retried request therefore carries
every header it added twice, including `Host`, `x-amz-date`,
`x-amz-content-sha256` and `Authorization`.

## Where

All paths are in `src/s3/client/mod.rs`. Line numbers are for `6db367b`. The
same code is at `c35e028`, at lines 731, 751, 822 and 1045.

| What                                                                                         | Line                         |
| -------------------------------------------------------------------------------------------- | ---------------------------- |
| `execute` calls `execute_internal` with `headers`, then again with the same `headers`        | 1059 to 1112 (retry at 1095) |
| `execute_internal` takes `headers: &mut Multimap`                                            | 781                          |
| It adds `Host`, content headers, `x-amz-date`, `x-amz-content-sha256` and the security token | 801 to 907                   |
| It runs the `before_signing_mut` hooks on that map                                           | 872                          |
| It adds `x-minio-redirect-from` and `x-minio-redirect-to` when a hook moved the URL          | 891, 892                     |
| The signer adds `Authorization` (`src/s3/signer.rs`)                                         | 339, 816                     |
| The request is built by appending every value of every key                                   | 946 to 950                   |

`Multimap` is `multimap::MultiMap<String, String>` (`src/s3/multimap_ext.rs:22`).
`MultimapExt::add` calls `insert`, which appends a value and never replaces one.
`req.header(key, value)` also appends. So a key added twice is sent twice.

## When it happens

A retry happens only when all of these hold:

- The method is `HEAD`.
- The call names a bucket that is already in `region_map`.
- The server answers `301`, `307` or `400` with an empty body and an
  `x-amz-bucket-region` header (`handle_redirect_response`, line 1470 onward).

That is a bucket whose region changed from what the client cached. AWS S3 does
this. AIStor does not send that answer, so a client that talks only to AIStor does
not reach it.

## What goes wrong

Verified by reading the code:

- The second attempt's map holds two values for `Host`, `x-amz-date`,
  `x-amz-content-sha256`, any hook header, and `Authorization`. The first
  `Authorization` was signed over the first attempt's date.
- The signer leaves `authorization` and `user-agent` out of the canonical
  headers, and it includes every other header. So the second signature covers
  both copies of each header.
- A hook that adds a header on every call adds it a second time. For example, a
  tracing hook that sets `traceparent` sends two `traceparent` headers on a retry. The W3C trace context rules let a
  receiver treat a request with more than one as having none.

Not verified, because it needs a server that answers the redirect:

- Whether S3 or MinIO accepts the second attempt, or answers
  `SignatureDoesNotMatch` or `InvalidArgument` for the duplicate `x-amz-date` and
  `Authorization`.

## How to reproduce

1. Build a client with a bucket in `region_map` (an earlier call that resolved
   its region).
2. Point it at a stub server that answers `HEAD /{bucket}` with `301`, an empty
   body and `x-amz-bucket-region: other-region` once, and then with `200`.
3. Add a request hook that sets a header, and record the headers the stub
   receives on the second request.
4. The second request shows two `Host`, `x-amz-date` and hook header values and
   two `Authorization` values.

## Why it matters little today, and why to fix it anyway

No known workload reaches the retry, so this is not urgent. The failure is hard
to read when it does occur: the server sees a malformed request, and the client
reports its error, not the redirect. Any user hook that is not idempotent also
runs twice on the same map.

## Direction (not done)

Each attempt should start from the caller's headers. Two ways to get there:

- Clone the `Multimap` before each call to `execute_internal`.
- Make `execute_internal` build its own working map from `&Multimap`.

Either needs a test like the reproduction above.

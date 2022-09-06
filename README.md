*** Currency Converter API ***

To start the application, run `cargo run`.

The API expects a `POST` request to `localhost:3000/currency`, along with a body similar to the example below.

`{
  "from": "mexico",
  "to": "france",
  amount: 3  
}`

This will return the following:

`[{"from":"MXN","to":"EUR","amount":0.2516408609387861}]`

Some countries, such as China, will return multiple currencies, in which case the response array will have multiple entries.
Nonexistent countries will return an error.

Rate limiting can be configured in the `main` file.

Tests for the country cache can be run using `cargo test`.

Country data is cached on first request, so that subsequent responses will be sourced from the cache rather than making an unnecessary request.

The primary bottlenecks arise from hitting the other endpoints.
Both the "from" and "to" country data requests are polled as one future via `try_join`, so one does not wait on the other.
Because the server is multithreaded, a `RwLock` is used, so that multiple readers can read from the cache at any point in time,
but only one can write at any point in time. Thus there is the chance that at certain times, one writer may have to wait for another
to complete its task before the other is able to write.
Some effort could be put into exploring multiple requesters requesting uncached data simultaneously, and making sure that
requests are not dropped, nor paused longer than necessary when this occurs.

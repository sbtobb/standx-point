- Use one long-lived blocking crossterm event reader that forwards Key presses into the app channel; per-tick spawn_blocking can drop input events.

- Tests: wiremock can hit "connection closed before message completed" if the mock server is dropped while background tasks are still issuing requests; waiting for expected request counts makes shutdown tests stable.

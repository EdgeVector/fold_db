Project Settings
- use ./run_http_server to start the server, it will kill all other db proccesses and start the front and back end.  It runs on port 9001.
- no custom url endpoints, all UI endpoints derived using OpenAPI

Coding standards.  Follow these to avoid creating spagetti code.
- No silent failures.  Throw errors if anything goes wrong.
- No branching logic -- think harder if it seems like there's going to be branching logic.
- Don't use JSON return types.
- Don't make assumptions that test failures were preexisting.  Assume all tests were passing before.
- use TODO format for anything not fully implemented.
- Don't use inline crate imports, import in headers only to keep code clean.
- Keep imports organized.
- Don't create fallbacks.
- validate your changes -- in the browser.

current UI test:
./run_http_server_dynamo_db.sh
navigate to http://localhost:9001
press reset database button -- confirm on the modal, wait for reset to complete
ingestion tab
click twitter
click Process Data
wait for ingestion to complete
indexing should run in the background
wait for indexing to complete
click on native index query tab
search for a term
it should show up.
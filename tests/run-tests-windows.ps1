# Set environment variables to run tests on play.min.io
$Env:SERVER_ENDPOINT = "http://localhost:9000/"
$Env:ACCESS_KEY = "henk"
$Env:SECRET_KEY = "Da4s88Uf!"
$Env:ENABLE_HTTPS = "false"
$Env:SSL_CERT_FILE = "./tests/public.crt"
$Env:IGNORE_CERT_CHECK = "false"
$Env:SERVER_REGION = ""

# Run tests
cargo test --features ring
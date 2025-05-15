# Set environment variables to run tests on play.min.io
$Env:SERVER_ENDPOINT = "http://localhost:9000/"
$Env:ACCESS_KEY = "minioadmin"
$Env:SECRET_KEY = "minioadmin"
$Env:ENABLE_HTTPS = "false"
$Env:MINIO_SSL_CERT_FILE = "./tests/public.crt"
$Env:IGNORE_CERT_CHECK = "false"
$Env:SERVER_REGION = ""


# Set environment variables for test_bucket_notification
$Env:MINIO_CI_CD = "true"
$Env:MINIO_NOTIFY_WEBHOOK_ENABLE_miniojavatest = "on"
$Env:MINIO_NOTIFY_WEBHOOK_ENDPOINT_miniojavatest = "http://example.org/"

# Run tests
# cargo test -- --nocapture


# run one specific test and show stdout
# cargo test --test test_bucket_replication -- --nocapture
cargo test --test test_bucket_notification -- --nocapture
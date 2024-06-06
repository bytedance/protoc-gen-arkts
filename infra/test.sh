# build
cargo build

# Generation of ArtTs file.
protoc -I /usr/local/include -I . --arkts_out=./ ./tests/gen.proto  --plugin=./target/debug/protoc-gen-arkts --arkts_opt=with_sendable=true

# Test generated ArtTs file.
# Wait till we get an ArkTs runtime from Huawei.
#
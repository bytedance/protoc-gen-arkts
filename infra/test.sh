# build
cargo build

# Generation of ArtTs file.
protoc -I /usr/local/include -I /Users/bytedance/work/protoc-gen-arkts/tests --arkts_out=/Users/bytedance/work/protoc-gen-arkts/tests /Users/bytedance/work/protoc-gen-arkts/tests/gen.proto  --plugin=./target/debug/protoc-gen-arkts --arkts_opt=with_sendable=true
protoc -I /usr/local/include -I /Users/bytedance/work/protoc-gen-arkts/tests --arkts_out=/Users/bytedance/work/protoc-gen-arkts/tests /Users/bytedance/work/protoc-gen-arkts/tests/common.proto  --plugin=./target/debug/protoc-gen-arkts --arkts_opt=with_sendable=true
protoc -I /usr/local/include -I /Users/bytedance/work/protoc-gen-arkts/tests --arkts_out=/Users/bytedance/work/protoc-gen-arkts/tests /Users/bytedance/work/protoc-gen-arkts/tests/enum.proto  --plugin=./target/debug/protoc-gen-arkts --arkts_opt=with_sendable=true

# Test generated ArtTs file.
# Wait till we get an ArkTs runtime from Huawei.
#
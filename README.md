
## Proctoc-gen-arkts
[![License](https://img.shields.io/badge/MIT-blue.svg)](https://opensource.org/license/MIT)


Compile `.proto` files to plain arkTs.




## Contributing
- the future is open to everyone


## Features

- Supports json encoding (`toJson`, `fromJson`)
- Supports binary encoding (`toBinary`, `fromBinary`)
- Supports Sendable

## Usage

### Protoc

```properties
protoc -I $proto_path --arkts_out=$output_path gen.proto  --plugin=target/debug/protoc-gen-arkts
```


### Example

```proto
syntax = "proto3";
package struct_pkg;

enum Enum {
    ENUM_0 = 0;
    ENUM_1 = 1;
}

message Common {
	string common_id = 1;
	int64 common_type = 2;
}

message Struct {
  string field_string = 1;
  repeated string field_string_list = 2;
  int32 field_i32 = 3;
  repeated int32 field_i32_list = 4;
  int64 field_i64 = 5;
  repeated int64 field_i64_list = 6;
  bool field_boolean = 7;
  repeated bool field_boolean_list = 8;

  bytes field_bytes = 9;
  repeated bytes field_bytes_list = 10;

  Enum field_enum = 11;
  repeated Enum field_enum_list = 12;

  Common field_common_struct = 13;
  repeated Common field_common_struct_list = 14;

  map<string, string> field_map_string_string = 15;
  map<string, int32> field_map_string_i32 = 16;
  map<int64, int64> field_map_i64_i64 = 17;
  map<int64, string> field_map_i64_string = 18;  
}


message Struct2 {
  uint32 field_uint32 = 1;
}
```


```arkts
 const struct_impl = new struct_pkg_Struct()
    struct_impl.field_string = "field string"
    struct_impl.field_string_list = ["field string list1", "field string list2"]
    struct_impl.field_i32 = 100
    struct_impl.field_i32_list = [1, 2, 3]
    struct_impl.field_i64 = BigInt(10)
    struct_impl.field_i64_list = [BigInt(1), BigInt(2), BigInt(3), BigInt(4), BigInt(5)]
    struct_impl.field_boolean = false
    struct_impl.field_boolean_list = [false, true, false, true]
    struct_impl.field_bytes = new Uint8Array(10)
    struct_impl.field_bytes.set([96, 97])
    struct_impl.field_bytes_list = []
    struct_impl.field_bytes_list.push(struct_impl.field_bytes)
    struct_impl.field_enum = struct_pkg_Enum.ENUM_1
    struct_impl.field_enum_list = [struct_pkg_Enum.ENUM_1, struct_pkg_Enum.ENUM_1]

    const common_impl = new struct_pkg_Common()
    common_impl.common_id = "id"
    common_impl.common_type = BigInt(100)
    const common_impl1 = new struct_pkg_Common()
    common_impl1.common_id = "id"
    common_impl1.common_type = BigInt(100)
    struct_impl.field_common_struct = common_impl
    struct_impl.field_common_struct_list = [common_impl, common_impl1]
    struct_impl.field_map_string_string = new Map<string, string>()
    struct_impl.field_map_string_i32 = new Map<string, number>()
    struct_impl.field_map_i64_i64 = new Map<bigint, bigint>()
    struct_impl.field_map_i64_string = new Map<bigint, string>()
    for (let i = 0; i < 10; ++i ) {
      struct_impl.field_map_string_string.set(i.toString(), i.toString())
      struct_impl.field_map_string_i32.set(i.toString(), i)
      struct_impl.field_map_i64_i64.set(BigInt(i), BigInt(i))
      struct_impl.field_map_i64_string.set(BigInt(i), i.toString())
    }

    // const binarybuf: ArrayBuffer = struct_impl.toBinary()
    // const new_struct_pkg_struct = struct_pkg_Struct.fromBinary(new Uint8Array(binarybuf))

    // to json & from from
    const new_to_json_obj = struct_impl.toJson();
    const new_from_json_struct = struct_pkg_Struct.fromJson(new_to_json_obj)

    // to binary & from binary
    const new_to_binary_buf = struct_impl.toBinary()
    const new_from_binary_struct = struct_pkg_Struct.fromBinary(new_to_binary_buf)

```

## Development

```sh
- cargo build
```
when the compilation is completed, the protoc-gen-arkts will appear in target/debug/


```harmony dependencies
- "google-protobuf": "3.21.2"
- "js-base64": "3.7.7"
```
add dependencies to the oh-package.json5 file in the project


## test

```sh
./infra/test.sh
```



## Thank
- Many thanks to the original author

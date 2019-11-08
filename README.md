# tls_read_hancock_bin

Reader for the Hancock terrestrial LiDAR binary polar format, see https://bitbucket.org/StevenHancock/libclidar.

## Example

`Cargo.toml`:
```toml
[dependencies]
tls_read_hancock_bin = "0.1.1"
```

And in your rust code:
```rust
extern crate hancock_read_bin;
use hancock_read_bin::HancockReader;

// [...]

let file_path_str = String::from("some_path.bin");
let mut beam_reader = HancockReader::new(file_path_str).unwrap();

println!("Number of beams: {}", beam_reader.n_beams);

while let Some(data) in beam_reader.into_iter() {
    println!("\r{:?}", data);
}

```
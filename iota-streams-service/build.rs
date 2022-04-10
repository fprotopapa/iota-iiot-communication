fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/iota_streams_grpc.proto")?;
    Ok(())
}

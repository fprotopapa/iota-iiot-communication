fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/iota_streams_grpc.proto")?;
    tonic_build::compile_protos("proto/iota_identity_grpc.proto")?;
    tonic_build::compile_protos("proto/mqtt_encoder.proto")?;
    tonic_build::compile_protos("proto/mqtt_grpc.proto")?;
    Ok(())
}

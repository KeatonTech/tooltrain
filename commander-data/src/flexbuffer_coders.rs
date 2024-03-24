use anyhow::Error;
use flexbuffers::{FlexbufferSerializer, Reader};
use serde::{Serialize, Deserialize};

pub trait CommanderCoder {
    type Value;

    fn type_string(&self) -> String;

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error>;

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error>;

    fn encode(&self, value: Self::Value) -> Result<Vec<u8>, Error> {
        let mut serializer = flexbuffers::FlexbufferSerializer::new();
        self.encode_to_serializer(&mut serializer, value)?;
        Ok(serializer.take_buffer())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Self::Value, Error> {
        let reader = Reader::get_root(bytes)?;
        self.decode_from_reader(reader)
    }
}

pub trait CommanderWireFormatCoder {
    type Value;
    type WireFormat: Serialize + for<'a> Deserialize<'a>;

    fn type_string_(&self) -> String;

    fn encode_to_wire_format(&self, value: Self::Value) -> Result<Self::WireFormat, Error>;

    fn decode_from_wire_format(&self, wire_format: Self::WireFormat) -> Result<Self::Value, Error>;
}

impl<D> CommanderCoder for D
where
    D: CommanderWireFormatCoder,
    D: Send + Sync
{
    type Value = D::Value;

    fn type_string(&self) -> String {
        D::type_string_(self)
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        self.encode_to_wire_format(value)?.serialize(serializer)?;
        Ok(())
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        self.decode_from_wire_format(D::WireFormat::deserialize(reader)?)
    }
}

pub trait CommanderPrimitiveCoder{
    type Value;
    fn type_string__(&self) -> &'static str;
}

impl<P> CommanderWireFormatCoder for P
where
    P: CommanderPrimitiveCoder,
    P::Value: Serialize,
    P::Value: for<'de> Deserialize<'de>,
{
    type Value = P::Value;
    type WireFormat = P::Value;

    fn type_string_(&self) -> String {
        self.type_string__().to_string()
    }

    fn encode_to_wire_format(&self, value: Self::Value) -> Result<Self::WireFormat, Error> {
        Ok(value)
    }

    fn decode_from_wire_format(&self, wire_format: Self::WireFormat) -> Result<Self::Value, Error> {
        Ok(wire_format)
    }
}

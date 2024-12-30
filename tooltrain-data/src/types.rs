use crate::flexbuffer_coders::*;
use anyhow::{anyhow, Error};
use derive_more::{Deref, From, IsVariant, TryInto, Unwrap};
use flexbuffers::{FlexbufferSerializer, Reader};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::BTreeMap, marker::PhantomData, path::PathBuf};

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainTriggerDataType {}

impl TooltrainPrimitiveCoder for TooltrainTriggerDataType {
    type Value = PhantomData<bool>;
    fn type_string__(&self) -> &'static str {
        "trigger"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainBooleanDataType {}

impl TooltrainPrimitiveCoder for TooltrainBooleanDataType {
    type Value = bool;
    fn type_string__(&self) -> &'static str {
        "boolean"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainNumberDataType {}

impl TooltrainPrimitiveCoder for TooltrainNumberDataType {
    type Value = f64;
    fn type_string__(&self) -> &'static str {
        "number"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainStringDataType {}

impl TooltrainPrimitiveCoder for TooltrainStringDataType {
    type Value = String;
    fn type_string__(&self) -> &'static str {
        "string"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainBytesDataType {}

impl TooltrainPrimitiveCoder for TooltrainBytesDataType {
    type Value = Vec<u8>;
    fn type_string__(&self) -> &'static str {
        "bytes"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainColorDataType {}

impl TooltrainPrimitiveCoder for TooltrainColorDataType {
    type Value = [u16; 4];
    fn type_string__(&self) -> &'static str {
        "color"
    }
}

#[derive(Clone, Debug, Deref, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsonString(String);

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainJsonDataType {}

impl TooltrainPrimitiveCoder for TooltrainJsonDataType {
    type Value = JsonString;
    fn type_string__(&self) -> &'static str {
        "json"
    }
}

#[derive(Clone, Debug, Deref, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SvgString(String);

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainSvgDataType {}

impl TooltrainPrimitiveCoder for TooltrainSvgDataType {
    type Value = SvgString;
    fn type_string__(&self) -> &'static str {
        "svg"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct TooltrainPathDataType {}

impl TooltrainWireFormatCoder for TooltrainPathDataType {
    type Value = PathBuf;
    type WireFormat = Vec<String>;

    fn type_string_(&self) -> String {
        "path".to_string()
    }

    fn encode_to_wire_format(&self, value: Self::Value) -> Result<Self::WireFormat, Error> {
        Ok(value
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect())
    }

    fn decode_from_wire_format(&self, wire_format: Self::WireFormat) -> Result<Self::Value, Error> {
        Ok(PathBuf::from_iter(wire_format))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TooltrainEnumVariant {
    name: String,
    ordinal: u32,
}

impl TooltrainEnumVariant {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Default, Debug)]
pub struct TooltrainEnumDataType {
    name: String,
    variants: Vec<TooltrainEnumVariant>,
}

impl TooltrainEnumDataType {
    pub fn new(name: String, variants: Vec<String>) -> Self {
        TooltrainEnumDataType {
            name,
            variants: variants
                .into_iter()
                .enumerate()
                .map(|(ordinal, name)| TooltrainEnumVariant {
                    name,
                    ordinal: ordinal as u32,
                })
                .collect(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn list_variants(&self) -> impl Iterator<Item = &str> {
        self.variants.iter().map(TooltrainEnumVariant::get_name)
    }

    pub fn get_variant(&self, name: &str) -> Option<TooltrainEnumVariant> {
        self.variants.iter().find(|v| v.name == name).cloned()
    }
}

impl TooltrainWireFormatCoder for TooltrainEnumDataType {
    type Value = TooltrainEnumVariant;
    type WireFormat = u32;

    fn type_string_(&self) -> String {
        format!(
            "enum {}<{}>",
            self.name,
            self.list_variants().collect::<Vec<&str>>().join(", ")
        )
    }

    fn encode_to_wire_format(&self, value: Self::Value) -> Result<Self::WireFormat, Error> {
        Ok(value.ordinal)
    }

    fn decode_from_wire_format(&self, wire_format: Self::WireFormat) -> Result<Self::Value, Error> {
        self.variants
            .iter()
            .find(|v| v.ordinal == wire_format)
            .ok_or(anyhow!("Unknown enum variant {}", wire_format))
            .cloned()
    }
}

#[derive(Clone, Debug, From, TryInto, IsVariant, Unwrap)]
pub enum TooltrainDataType {
    Trigger(TooltrainTriggerDataType),
    Boolean(TooltrainBooleanDataType),
    Number(TooltrainNumberDataType),
    String(TooltrainStringDataType),
    Bytes(TooltrainBytesDataType),
    Color(TooltrainColorDataType),
    Json(TooltrainJsonDataType),
    Svg(TooltrainSvgDataType),
    Path(TooltrainPathDataType),
    Enum(TooltrainEnumDataType),
    Struct(TooltrainStructDataType),
    List(TooltrainListDataType),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, From, TryInto, IsVariant, Unwrap)]
pub enum TooltrainValue {
    Trigger(<TooltrainTriggerDataType as TooltrainCoder>::Value),
    Boolean(<TooltrainBooleanDataType as TooltrainCoder>::Value),
    Number(<TooltrainNumberDataType as TooltrainCoder>::Value),
    String(<TooltrainStringDataType as TooltrainCoder>::Value),
    Bytes(<TooltrainBytesDataType as TooltrainCoder>::Value),
    Color(<TooltrainColorDataType as TooltrainCoder>::Value),
    Json(<TooltrainJsonDataType as TooltrainCoder>::Value),
    Svg(<TooltrainSvgDataType as TooltrainCoder>::Value),
    Path(<TooltrainPathDataType as TooltrainCoder>::Value),
    Enum(<TooltrainEnumDataType as TooltrainCoder>::Value),
    Struct(<TooltrainStructDataType as TooltrainCoder>::Value),
    List(<TooltrainListDataType as TooltrainCoder>::Value),
}

impl TooltrainCoder for TooltrainDataType {
    type Value = TooltrainValue;

    fn type_string(&self) -> String {
        match self {
            TooltrainDataType::Trigger(inner) => inner.type_string(),
            TooltrainDataType::Boolean(inner) => inner.type_string(),
            TooltrainDataType::Number(inner) => inner.type_string(),
            TooltrainDataType::String(inner) => inner.type_string(),
            TooltrainDataType::Bytes(inner) => inner.type_string(),
            TooltrainDataType::Color(inner) => inner.type_string(),
            TooltrainDataType::Json(inner) => inner.type_string(),
            TooltrainDataType::Svg(inner) => inner.type_string(),
            TooltrainDataType::Path(inner) => inner.type_string(),
            TooltrainDataType::Enum(inner) => inner.type_string(),
            TooltrainDataType::Struct(inner) => inner.type_string(),
            TooltrainDataType::List(inner) => inner.type_string(),
        }
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        match self {
            TooltrainDataType::Trigger(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a trigger value. {s}"))?,
            ),
            TooltrainDataType::Boolean(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a boolean value. {s}"))?,
            ),
            TooltrainDataType::Number(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a number value. {s}"))?,
            ),
            TooltrainDataType::String(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a string value. {s}"))?,
            ),
            TooltrainDataType::Bytes(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a bytes value. {s}"))?,
            ),
            TooltrainDataType::Color(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a color value. {s}"))?,
            ),
            TooltrainDataType::Json(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a json value. {s}"))?,
            ),
            TooltrainDataType::Svg(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a svg value. {s}"))?,
            ),
            TooltrainDataType::Path(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a path value. {s}"))?,
            ),
            TooltrainDataType::Enum(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a enum value. {s}"))?,
            ),
            TooltrainDataType::Struct(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a struct value. {s}"))?,
            ),
            TooltrainDataType::List(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a list value. {s}"))?,
            ),
        }
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        match self {
            TooltrainDataType::Trigger(inner) => {
                Ok(TooltrainValue::Trigger(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Boolean(inner) => {
                Ok(TooltrainValue::Boolean(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Number(inner) => {
                Ok(TooltrainValue::Number(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::String(inner) => {
                Ok(TooltrainValue::String(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Bytes(inner) => {
                Ok(TooltrainValue::Bytes(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Color(inner) => {
                Ok(TooltrainValue::Color(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Json(inner) => {
                Ok(TooltrainValue::Json(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Svg(inner) => {
                Ok(TooltrainValue::Svg(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Path(inner) => {
                Ok(TooltrainValue::Path(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Enum(inner) => {
                Ok(TooltrainValue::Enum(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::Struct(inner) => {
                Ok(TooltrainValue::Struct(inner.decode_from_reader(reader)?))
            }
            TooltrainDataType::List(inner) => {
                Ok(TooltrainValue::List(inner.decode_from_reader(reader)?))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TooltrainStructDataType {
    pub name: String,
    field_names: Vec<String>,
    field_types: Vec<TooltrainDataType>,
}

impl TooltrainStructDataType {
    pub fn column_types(&self) -> Vec<String> {
        self.field_types.iter().map(|t| t.type_string()).collect()
    }
}

#[derive(Clone)]
pub struct TooltrainStructTypeBuilder {
    pub name: String,
    field_names: Vec<String>,
    field_types: Vec<TooltrainDataType>,
}

impl TooltrainStructTypeBuilder {
    pub fn new(name: &str) -> Self {
        TooltrainStructTypeBuilder {
            name: name.to_string(),
            field_names: vec![],
            field_types: vec![],
        }
    }

    pub fn add_field<D>(mut self, name: &str, data_type: D) -> Self
    where
        D: 'static,
        D: TooltrainCoder,
        D: Into<TooltrainDataType>,
    {
        self.field_names.push(name.to_string());
        self.field_types.push(data_type.into());
        self
    }

    pub fn build(self) -> TooltrainStructDataType {
        TooltrainStructDataType {
            name: self.name,
            field_names: self.field_names,
            field_types: self.field_types,
        }
    }
}

impl TooltrainCoder for TooltrainStructDataType {
    type Value = BTreeMap<String, TooltrainValue>;

    fn type_string(&self) -> String {
        let type_args = self
            .field_names
            .iter()
            .zip(self.field_types.iter())
            .map(|(name, type_box)| format!("{}: {}", name, type_box.type_string()))
            .collect::<Vec<String>>()
            .join(", ");
        format!("struct {}<{}>", self.name, type_args)
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        let seq_serializer = serializer.serialize_seq(Some(self.field_names.len()))?;

        for ((_, value), type_box) in value.into_iter().zip(self.field_types.iter()) {
            type_box.encode_to_serializer(seq_serializer, value)?;
        }

        seq_serializer.end()?;
        Ok(())
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        let vector_reader = reader.get_vector()?;
        let mut values: Vec<TooltrainValue> = vec![];
        for (reader, type_box) in vector_reader.iter().zip(self.field_types.iter()) {
            values.push(type_box.decode_from_reader(reader)?);
        }
        Ok(self.field_names.clone().into_iter().zip(values).collect())
    }
}

#[derive(Clone, Debug)]
pub struct TooltrainTypedListDataType<V: TooltrainCoder + 'static> {
    child_type: V,
}

impl<V: TooltrainCoder + 'static> TooltrainTypedListDataType<V> {
    pub fn new(child_type: V) -> Self {
        TooltrainTypedListDataType::<V> { child_type }
    }
}

impl<V: TooltrainCoder + 'static> TooltrainCoder for TooltrainTypedListDataType<V> {
    type Value = Vec<V::Value>;

    fn type_string(&self) -> String {
        format!("list<{}>", self.child_type.type_string())
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        let seq_serializer = serializer.serialize_seq(Some(value.len()))?;

        for row in value {
            self.child_type.encode_to_serializer(seq_serializer, row)?;
        }

        seq_serializer.end()?;
        Ok(())
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        let vector_reader = reader.get_vector()?;
        let mut values: Vec<V::Value> = vec![];
        for reader in vector_reader.iter() {
            values.push(self.child_type.decode_from_reader(reader)?);
        }
        Ok(values)
    }
}

pub type TooltrainGenericListDataType = TooltrainTypedListDataType<TooltrainDataType>;

#[derive(Clone, Debug, TryInto, IsVariant, Unwrap)]
pub enum TooltrainListDataType {
    Boolean(TooltrainTypedListDataType<TooltrainBooleanDataType>),
    Number(TooltrainTypedListDataType<TooltrainNumberDataType>),
    String(TooltrainTypedListDataType<TooltrainStringDataType>),
    Bytes(TooltrainTypedListDataType<TooltrainBytesDataType>),
    Color(TooltrainTypedListDataType<TooltrainColorDataType>),
    Json(TooltrainTypedListDataType<TooltrainJsonDataType>),
    Svg(TooltrainTypedListDataType<TooltrainSvgDataType>),
    Path(TooltrainTypedListDataType<TooltrainPathDataType>),
    Enum(TooltrainTypedListDataType<TooltrainEnumDataType>),
    Struct(TooltrainTypedListDataType<TooltrainStructDataType>),
    Generic(Box<TooltrainGenericListDataType>),
}

#[derive(Clone, Debug, TryInto, IsVariant, Unwrap)]
pub enum TooltrainListValue {
    Boolean(Vec<<TooltrainBooleanDataType as TooltrainCoder>::Value>),
    Number(Vec<<TooltrainNumberDataType as TooltrainCoder>::Value>),
    String(Vec<<TooltrainStringDataType as TooltrainCoder>::Value>),
    Bytes(Vec<<TooltrainBytesDataType as TooltrainCoder>::Value>),
    Color(Vec<<TooltrainColorDataType as TooltrainCoder>::Value>),
    Json(Vec<<TooltrainJsonDataType as TooltrainCoder>::Value>),
    Svg(Vec<<TooltrainSvgDataType as TooltrainCoder>::Value>),
    Path(Vec<<TooltrainPathDataType as TooltrainCoder>::Value>),
    Enum(Vec<<TooltrainEnumDataType as TooltrainCoder>::Value>),
    Struct(Vec<<TooltrainStructDataType as TooltrainCoder>::Value>),
    Generic(Vec<Box<TooltrainValue>>),
}

impl TooltrainCoder for TooltrainListDataType {
    type Value = Vec<TooltrainValue>;

    fn type_string(&self) -> String {
        match self {
            TooltrainListDataType::Boolean(inner) => inner.type_string(),
            TooltrainListDataType::Number(inner) => inner.type_string(),
            TooltrainListDataType::String(inner) => inner.type_string(),
            TooltrainListDataType::Bytes(inner) => inner.type_string(),
            TooltrainListDataType::Color(inner) => inner.type_string(),
            TooltrainListDataType::Json(inner) => inner.type_string(),
            TooltrainListDataType::Svg(inner) => inner.type_string(),
            TooltrainListDataType::Path(inner) => inner.type_string(),
            TooltrainListDataType::Enum(inner) => inner.type_string(),
            TooltrainListDataType::Struct(inner) => inner.type_string(),
            TooltrainListDataType::Generic(inner) => inner.type_string(),
        }
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        match self {
            TooltrainListDataType::Boolean(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Number(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::String(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Bytes(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Color(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Json(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Svg(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Path(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Enum(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Struct(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            TooltrainListDataType::Generic(inner) => inner.encode_to_serializer(serializer, value),
        }
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        match self {
            TooltrainListDataType::Boolean(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Number(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::String(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Bytes(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Color(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Json(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Svg(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Path(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Enum(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Struct(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            TooltrainListDataType::Generic(inner) => inner.decode_from_reader(reader),
        }
    }
}

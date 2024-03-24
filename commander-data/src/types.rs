use crate::flexbuffer_coders::*;
use anyhow::{anyhow, Error};
use derive_more::{Deref, From, IsVariant, TryInto, Unwrap};
use flexbuffers::{FlexbufferSerializer, Reader};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::BTreeMap, marker::PhantomData, path::PathBuf};

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderTriggerDataType {}

impl CommanderPrimitiveCoder for CommanderTriggerDataType {
    type Value = PhantomData<bool>;
    fn type_string__(&self) -> &'static str {
        "trigger"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderBooleanDataType {}

impl CommanderPrimitiveCoder for CommanderBooleanDataType {
    type Value = bool;
    fn type_string__(&self) -> &'static str {
        "boolean"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderNumberDataType {}

impl CommanderPrimitiveCoder for CommanderNumberDataType {
    type Value = f64;
    fn type_string__(&self) -> &'static str {
        "number"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderStringDataType {}

impl CommanderPrimitiveCoder for CommanderStringDataType {
    type Value = String;
    fn type_string__(&self) -> &'static str {
        "string"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderBytesDataType {}

impl CommanderPrimitiveCoder for CommanderBytesDataType {
    type Value = Vec<u8>;
    fn type_string__(&self) -> &'static str {
        "bytes"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderColorDataType {}

impl CommanderPrimitiveCoder for CommanderColorDataType {
    type Value = [u16; 4];
    fn type_string__(&self) -> &'static str {
        "color"
    }
}

#[derive(Clone, Debug, Deref, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct JsonString(String);

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderJsonDataType {}

impl CommanderPrimitiveCoder for CommanderJsonDataType {
    type Value = JsonString;
    fn type_string__(&self) -> &'static str {
        "json"
    }
}

#[derive(Clone, Debug, Deref, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SvgString(String);

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderSvgDataType {}

impl CommanderPrimitiveCoder for CommanderSvgDataType {
    type Value = SvgString;
    fn type_string__(&self) -> &'static str {
        "svg"
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct CommanderPathDataType {}

impl CommanderWireFormatCoder for CommanderPathDataType {
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
pub struct CommanderEnumVariant {
    name: String,
    ordinal: u32,
}

impl CommanderEnumVariant {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Default, Debug)]
pub struct CommanderEnumDataType {
    name: String,
    variants: Vec<CommanderEnumVariant>,
}

impl CommanderEnumDataType {
    pub fn new(name: String, variants: Vec<String>) -> Self {
        CommanderEnumDataType {
            name,
            variants: variants
                .into_iter()
                .enumerate()
                .map(|(ordinal, name)| CommanderEnumVariant {
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
        self.variants.iter().map(CommanderEnumVariant::get_name)
    }

    pub fn get_variant(&self, name: &str) -> Option<CommanderEnumVariant> {
        self.variants.iter().find(|v| v.name == name).cloned()
    }
}

impl CommanderWireFormatCoder for CommanderEnumDataType {
    type Value = CommanderEnumVariant;
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
            .map(|v| v.clone())
    }
}

#[derive(Clone, Debug, From, TryInto, IsVariant, Unwrap)]
pub enum CommanderDataType {
    Trigger(CommanderTriggerDataType),
    Boolean(CommanderBooleanDataType),
    Number(CommanderNumberDataType),
    String(CommanderStringDataType),
    Bytes(CommanderBytesDataType),
    Color(CommanderColorDataType),
    Json(CommanderJsonDataType),
    Svg(CommanderSvgDataType),
    Path(CommanderPathDataType),
    Enum(CommanderEnumDataType),
    Struct(CommanderStructDataType),
    List(CommanderListDataType),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, From, TryInto, IsVariant, Unwrap)]
pub enum CommanderValue {
    Trigger(<CommanderTriggerDataType as CommanderCoder>::Value),
    Boolean(<CommanderBooleanDataType as CommanderCoder>::Value),
    Number(<CommanderNumberDataType as CommanderCoder>::Value),
    String(<CommanderStringDataType as CommanderCoder>::Value),
    Bytes(<CommanderBytesDataType as CommanderCoder>::Value),
    Color(<CommanderColorDataType as CommanderCoder>::Value),
    Json(<CommanderJsonDataType as CommanderCoder>::Value),
    Svg(<CommanderSvgDataType as CommanderCoder>::Value),
    Path(<CommanderPathDataType as CommanderCoder>::Value),
    Enum(<CommanderEnumDataType as CommanderCoder>::Value),
    Struct(<CommanderStructDataType as CommanderCoder>::Value),
    List(<CommanderListDataType as CommanderCoder>::Value),
}

impl CommanderCoder for CommanderDataType {
    type Value = CommanderValue;

    fn type_string(&self) -> String {
        match self {
            CommanderDataType::Trigger(inner) => inner.type_string(),
            CommanderDataType::Boolean(inner) => inner.type_string(),
            CommanderDataType::Number(inner) => inner.type_string(),
            CommanderDataType::String(inner) => inner.type_string(),
            CommanderDataType::Bytes(inner) => inner.type_string(),
            CommanderDataType::Color(inner) => inner.type_string(),
            CommanderDataType::Json(inner) => inner.type_string(),
            CommanderDataType::Svg(inner) => inner.type_string(),
            CommanderDataType::Path(inner) => inner.type_string(),
            CommanderDataType::Enum(inner) => inner.type_string(),
            CommanderDataType::Struct(inner) => inner.type_string(),
            CommanderDataType::List(inner) => inner.type_string(),
        }
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        match self {
            CommanderDataType::Trigger(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a trigger value. {s}"))?,
            ),
            CommanderDataType::Boolean(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a boolean value. {s}"))?,
            ),
            CommanderDataType::Number(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a number value. {s}"))?,
            ),
            CommanderDataType::String(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a string value. {s}"))?,
            ),
            CommanderDataType::Bytes(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a bytes value. {s}"))?,
            ),
            CommanderDataType::Color(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a color value. {s}"))?,
            ),
            CommanderDataType::Json(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a json value. {s}"))?,
            ),
            CommanderDataType::Svg(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a svg value. {s}"))?,
            ),
            CommanderDataType::Path(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a path value. {s}"))?,
            ),
            CommanderDataType::Enum(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a enum value. {s}"))?,
            ),
            CommanderDataType::Struct(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a struct value. {s}"))?,
            ),
            CommanderDataType::List(inner) => inner.encode_to_serializer(
                serializer,
                value
                    .try_into()
                    .map_err(|s| anyhow!("Expected a list value. {s}"))?,
            ),
        }
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        match self {
            CommanderDataType::Trigger(inner) => {
                Ok(CommanderValue::Trigger(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Boolean(inner) => {
                Ok(CommanderValue::Boolean(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Number(inner) => {
                Ok(CommanderValue::Number(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::String(inner) => {
                Ok(CommanderValue::String(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Bytes(inner) => {
                Ok(CommanderValue::Bytes(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Color(inner) => {
                Ok(CommanderValue::Color(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Json(inner) => {
                Ok(CommanderValue::Json(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Svg(inner) => {
                Ok(CommanderValue::Svg(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Path(inner) => {
                Ok(CommanderValue::Path(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Enum(inner) => {
                Ok(CommanderValue::Enum(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::Struct(inner) => {
                Ok(CommanderValue::Struct(inner.decode_from_reader(reader)?))
            }
            CommanderDataType::List(inner) => {
                Ok(CommanderValue::List(inner.decode_from_reader(reader)?))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommanderStructDataType {
    pub name: String,
    field_names: Vec<String>,
    field_types: Vec<CommanderDataType>,
}

impl CommanderStructDataType {
    pub fn column_types(&self) -> Vec<String> {
        self.field_types.iter().map(|t| t.type_string()).collect()
    }
}

#[derive(Clone)]
pub struct CommanderStructTypeBuilder {
    pub name: String,
    field_names: Vec<String>,
    field_types: Vec<CommanderDataType>,
}

impl CommanderStructTypeBuilder {
    pub fn new(name: &str) -> Self {
        CommanderStructTypeBuilder {
            name: name.to_string(),
            field_names: vec![],
            field_types: vec![],
        }
    }

    pub fn add_field<D>(mut self, name: &str, data_type: D) -> Self
    where
        D: 'static,
        D: CommanderCoder,
        D: Into<CommanderDataType>,
    {
        self.field_names.push(name.to_string());
        self.field_types.push(data_type.into());
        self
    }

    pub fn build(self) -> CommanderStructDataType {
        CommanderStructDataType {
            name: self.name,
            field_names: self.field_names,
            field_types: self.field_types,
        }
    }
}

impl CommanderCoder for CommanderStructDataType {
    type Value = BTreeMap<String, CommanderValue>;

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
        let mut values: Vec<CommanderValue> = vec![];
        for (reader, type_box) in vector_reader.iter().zip(self.field_types.iter()) {
            values.push(type_box.decode_from_reader(reader)?);
        }
        Ok(self.field_names.clone().into_iter().zip(values).collect())
    }
}

#[derive(Clone, Debug)]
pub struct CommanderTypedListDataType<V: CommanderCoder + 'static> {
    child_type: V,
}

impl<V: CommanderCoder + 'static> CommanderTypedListDataType<V> {
    pub fn new(child_type: V) -> Self {
        CommanderTypedListDataType::<V> { child_type }
    }
}

impl<V: CommanderCoder + 'static> CommanderCoder for CommanderTypedListDataType<V> {
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

pub type CommanderGenericListDataType = CommanderTypedListDataType<CommanderDataType>;

#[derive(Clone, Debug, TryInto, IsVariant, Unwrap)]
pub enum CommanderListDataType {
    Boolean(CommanderTypedListDataType<CommanderBooleanDataType>),
    Number(CommanderTypedListDataType<CommanderNumberDataType>),
    String(CommanderTypedListDataType<CommanderStringDataType>),
    Bytes(CommanderTypedListDataType<CommanderBytesDataType>),
    Color(CommanderTypedListDataType<CommanderColorDataType>),
    Json(CommanderTypedListDataType<CommanderJsonDataType>),
    Svg(CommanderTypedListDataType<CommanderSvgDataType>),
    Path(CommanderTypedListDataType<CommanderPathDataType>),
    Enum(CommanderTypedListDataType<CommanderEnumDataType>),
    Struct(CommanderTypedListDataType<CommanderStructDataType>),
    Generic(Box<CommanderGenericListDataType>),
}

#[derive(Clone, Debug, TryInto, IsVariant, Unwrap)]
pub enum CommanderListValue {
    Boolean(Vec<<CommanderBooleanDataType as CommanderCoder>::Value>),
    Number(Vec<<CommanderNumberDataType as CommanderCoder>::Value>),
    String(Vec<<CommanderStringDataType as CommanderCoder>::Value>),
    Bytes(Vec<<CommanderBytesDataType as CommanderCoder>::Value>),
    Color(Vec<<CommanderColorDataType as CommanderCoder>::Value>),
    Json(Vec<<CommanderJsonDataType as CommanderCoder>::Value>),
    Svg(Vec<<CommanderSvgDataType as CommanderCoder>::Value>),
    Path(Vec<<CommanderPathDataType as CommanderCoder>::Value>),
    Enum(Vec<<CommanderEnumDataType as CommanderCoder>::Value>),
    Struct(Vec<<CommanderStructDataType as CommanderCoder>::Value>),
    Generic(Vec<Box<CommanderValue>>),
}

impl CommanderCoder for CommanderListDataType {
    type Value = Vec<CommanderValue>;

    fn type_string(&self) -> String {
        match self {
            CommanderListDataType::Boolean(inner) => inner.type_string(),
            CommanderListDataType::Number(inner) => inner.type_string(),
            CommanderListDataType::String(inner) => inner.type_string(),
            CommanderListDataType::Bytes(inner) => inner.type_string(),
            CommanderListDataType::Color(inner) => inner.type_string(),
            CommanderListDataType::Json(inner) => inner.type_string(),
            CommanderListDataType::Svg(inner) => inner.type_string(),
            CommanderListDataType::Path(inner) => inner.type_string(),
            CommanderListDataType::Enum(inner) => inner.type_string(),
            CommanderListDataType::Struct(inner) => inner.type_string(),
            CommanderListDataType::Generic(inner) => inner.type_string(),
        }
    }

    fn encode_to_serializer(
        &self,
        serializer: &mut FlexbufferSerializer,
        value: Self::Value,
    ) -> Result<(), Error> {
        match self {
            CommanderListDataType::Boolean(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Number(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::String(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Bytes(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Color(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Json(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Svg(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Path(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Enum(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Struct(inner) => inner.encode_to_serializer(
                serializer,
                value.into_iter().map(|v| v.try_into().unwrap()).collect(),
            ),
            CommanderListDataType::Generic(inner) => inner.encode_to_serializer(serializer, value),
        }
    }

    fn decode_from_reader(&self, reader: Reader<&[u8]>) -> Result<Self::Value, Error> {
        match self {
            CommanderListDataType::Boolean(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Number(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::String(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Bytes(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Color(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Json(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Svg(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Path(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Enum(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Struct(inner) => Ok(inner
                .decode_from_reader(reader)?
                .into_iter()
                .map(|v| v.into())
                .collect()),
            CommanderListDataType::Generic(inner) => inner.decode_from_reader(reader),
        }
    }
}

use std::{marker::PhantomData, path::PathBuf};

use crate::{
    flexbuffer_coders::TooltrainPrimitiveCoder, JsonString, SvgString, TooltrainBooleanDataType,
    TooltrainBytesDataType, TooltrainColorDataType, TooltrainDataType, TooltrainEnumVariant,
    TooltrainJsonDataType, TooltrainListDataType, TooltrainNumberDataType, TooltrainStringDataType,
    TooltrainSvgDataType, TooltrainValue,
};

pub trait ConvertTooltrainValue {
    fn get_converter_fn(to_type: TooltrainDataType)
        -> Option<fn(TooltrainValue) -> TooltrainValue>;
}

impl ConvertTooltrainValue for PhantomData<bool> {
    fn get_converter_fn(_: TooltrainDataType) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        None
    }
}

impl ConvertTooltrainValue for bool {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Boolean(_) => Some(|value| value),
            TooltrainDataType::Number(_) => {
                Some(|value| TooltrainValue::Number(value.unwrap_boolean() as u8 as f64))
            }
            TooltrainDataType::String(_) => {
                Some(|value| TooltrainValue::String(value.unwrap_boolean().to_string()))
            }
            TooltrainDataType::Bytes(_) => {
                Some(|value| TooltrainValue::Bytes(vec![value.unwrap_boolean() as u8]))
            }
            TooltrainDataType::Json(_) => Some(|value| {
                TooltrainValue::Json(JsonString::from(value.unwrap_boolean().to_string()))
            }),
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for f64 {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Number(_) => Some(|value| value),
            TooltrainDataType::String(_) => {
                Some(|value| TooltrainValue::String(value.unwrap_number().to_string()))
            }
            TooltrainDataType::Json(_) => Some(|value| {
                TooltrainValue::Json(JsonString::from(value.unwrap_number().to_string()))
            }),
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for String {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::String(_) => Some(|value| value),
            TooltrainDataType::Bytes(_) => {
                Some(|value| TooltrainValue::Bytes(value.unwrap_string().bytes().collect()))
            }
            TooltrainDataType::Json(_) => {
                Some(|value| TooltrainValue::Json(JsonString::from(value.unwrap_string())))
            }
            TooltrainDataType::Svg(_) => {
                Some(|value| TooltrainValue::Svg(SvgString::from(value.unwrap_string())))
            }
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for Vec<u8> {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Bytes(_) => Some(|value| value),
            TooltrainDataType::String(_) => Some(|value| {
                TooltrainValue::String(String::from_utf8_lossy(&value.unwrap_bytes()).to_string())
            }),
            TooltrainDataType::List(TooltrainListDataType::Number(_)) => Some(|value| {
                TooltrainValue::List(
                    value
                        .unwrap_bytes()
                        .iter()
                        .map(|&v| TooltrainValue::Number(v as f64))
                        .collect(),
                )
            }),
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for [u16; 4] {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Color(_) => Some(|value| value),
            TooltrainDataType::List(TooltrainListDataType::Number(_)) => Some(|value| {
                TooltrainValue::List(
                    value
                        .unwrap_color()
                        .iter()
                        .map(|&v| TooltrainValue::Number(v as f64))
                        .collect(),
                )
            }),
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for JsonString {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Json(_) => Some(|value| value),
            TooltrainDataType::String(_) => {
                Some(|value| TooltrainValue::String(value.unwrap_json().to_string()))
            }
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for SvgString {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Svg(_) => Some(|value| value),
            TooltrainDataType::String(_) => {
                Some(|value| TooltrainValue::String(value.unwrap_svg().to_string()))
            }
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for PathBuf {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Path(_) => Some(|value| value),
            TooltrainDataType::String(_) => Some(|value| {
                TooltrainValue::String(value.unwrap_path().to_string_lossy().to_string())
            }),
            _ => None,
        }
    }
}

impl ConvertTooltrainValue for TooltrainEnumVariant {
    fn get_converter_fn(
        to_type: TooltrainDataType,
    ) -> Option<fn(TooltrainValue) -> TooltrainValue> {
        match to_type {
            TooltrainDataType::Enum(_) => Some(|value| value),
            _ => None,
        }
    }
}

pub trait CanConvertTooltrainValue {
    fn can_convert_to(&self, to_type: TooltrainDataType) -> bool;
}

impl<T: TooltrainPrimitiveCoder> CanConvertTooltrainValue for T {
    fn can_convert_to(&self, to_type: TooltrainDataType) -> bool {
        T::Value::get_converter_fn(to_type).is_some()
    }
}

impl CanConvertTooltrainValue for TooltrainDataType {
    fn can_convert_to(&self, to_type: TooltrainDataType) -> bool {
        match self {
            TooltrainDataType::Boolean(_) => {
                <TooltrainBooleanDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(
                    to_type,
                )
                .is_some()
            }
            TooltrainDataType::Number(_) => {
                <TooltrainNumberDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(
                    to_type,
                )
                .is_some()
            }
            TooltrainDataType::String(_) => {
                <TooltrainStringDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(
                    to_type,
                )
                .is_some()
            }
            TooltrainDataType::Bytes(_) => {
                <TooltrainBytesDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(
                    to_type,
                )
                .is_some()
            }
            TooltrainDataType::Color(_) => {
                <TooltrainColorDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(
                    to_type,
                )
                .is_some()
            }
            TooltrainDataType::Json(_) => {
                <TooltrainJsonDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(to_type)
                    .is_some()
            }
            TooltrainDataType::Svg(_) => {
                <TooltrainSvgDataType as TooltrainPrimitiveCoder>::Value::get_converter_fn(to_type)
                    .is_some()
            }
            TooltrainDataType::Path(_) => PathBuf::get_converter_fn(to_type).is_some(),
            TooltrainDataType::Enum(_) => TooltrainEnumVariant::get_converter_fn(to_type).is_some(),
            _ => false,
        }
    }
}

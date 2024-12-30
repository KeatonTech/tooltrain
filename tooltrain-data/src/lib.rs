use anyhow::{anyhow, Error};
use pest::{iterators::Pairs, Parser};
use pest_derive::Parser;

mod flexbuffer_coders;
pub mod types;

pub use flexbuffer_coders::TooltrainCoder;
pub use types::*;

#[derive(Parser)]
#[grammar = "../../wit/types.pest"] // relative to src
struct TypeParser;

pub fn parse(input: &str) -> Result<TooltrainDataType, Error> {
    let pairs = TypeParser::parse(Rule::r#type, input)?;
    expand_type(pairs)
}

fn expand_type(mut pairs: Pairs<'_, Rule>) -> Result<TooltrainDataType, Error> {
    match pairs.peek().ok_or(anyhow!("No type found"))?.as_rule() {
        Rule::trigger => {
            pairs.next().unwrap();
            Ok(TooltrainTriggerDataType {}.into())
        }
        _ => expand_static_type(pairs),
    }
}

fn expand_static_type(mut pairs: Pairs<'_, Rule>) -> Result<TooltrainDataType, Error> {
    match pairs
        .peek()
        .ok_or(anyhow!("No static_type found"))?
        .as_rule()
    {
        Rule::list => Ok(expand_list_type(pairs.next().unwrap().into_inner())?.into()),
        Rule::set => todo!(),
        Rule::map => todo!(),
        Rule::r#enum => Ok(expand_enum_type(pairs.next().unwrap().into_inner())?.into()),
        Rule::tuple => todo!(),
        Rule::r#struct => todo!(),
        _ => expand_primitive_type(pairs),
    }
}

fn expand_primitive_type(mut pairs: Pairs<'_, Rule>) -> Result<TooltrainDataType, Error> {
    match pairs
        .next()
        .ok_or(anyhow!("No primitive_type found"))?
        .as_rule()
    {
        Rule::boolean => Ok(TooltrainBooleanDataType {}.into()),
        Rule::number => Ok(TooltrainNumberDataType {}.into()),
        Rule::string => Ok(TooltrainStringDataType {}.into()),
        Rule::bytes => Ok(TooltrainBytesDataType {}.into()),
        Rule::color => Ok(TooltrainColorDataType {}.into()),
        Rule::path => Ok(TooltrainPathDataType {}.into()),
        Rule::url => todo!(),
        Rule::json => Ok(TooltrainJsonDataType {}.into()),
        Rule::svg => Ok(TooltrainSvgDataType {}.into()),
        _ => unreachable!(),
    }
}

fn expand_enum_type(mut pairs: Pairs<'_, Rule>) -> Result<TooltrainEnumDataType, Error> {
    let type_name_pair = pairs.next().unwrap();
    assert_eq!(Rule::type_name, type_name_pair.as_rule());
    let type_name = type_name_pair.as_str().to_string();

    let mut variants: Vec<String> = vec![];
    while let Some(Rule::enum_variant) = pairs.peek().map(|pair| pair.as_rule()) {
        variants.push(pairs.next().unwrap().as_str().to_string());
    }

    Ok(TooltrainEnumDataType::new(type_name, variants))
}

fn expand_list_type(pairs: Pairs<'_, Rule>) -> Result<TooltrainListDataType, Error> {
    let child_type = expand_static_type(pairs)?;
    match child_type {
        TooltrainDataType::Boolean(boolean_type) => Ok(TooltrainListDataType::Boolean(
            TooltrainTypedListDataType::new(boolean_type),
        )),
        TooltrainDataType::Number(number_type) => Ok(TooltrainListDataType::Number(
            TooltrainTypedListDataType::new(number_type),
        )),
        TooltrainDataType::String(string_type) => Ok(TooltrainListDataType::String(
            TooltrainTypedListDataType::new(string_type),
        )),
        TooltrainDataType::Bytes(bytes_type) => Ok(TooltrainListDataType::Bytes(
            TooltrainTypedListDataType::new(bytes_type),
        )),
        TooltrainDataType::Color(color_type) => Ok(TooltrainListDataType::Color(
            TooltrainTypedListDataType::new(color_type),
        )),
        TooltrainDataType::Json(json_type) => Ok(TooltrainListDataType::Json(
            TooltrainTypedListDataType::new(json_type),
        )),
        TooltrainDataType::Svg(svg_type) => Ok(TooltrainListDataType::Svg(
            TooltrainTypedListDataType::new(svg_type),
        )),
        TooltrainDataType::Path(path_type) => Ok(TooltrainListDataType::Path(
            TooltrainTypedListDataType::new(path_type),
        )),
        TooltrainDataType::Enum(enum_type) => Ok(TooltrainListDataType::Enum(
            TooltrainTypedListDataType::new(enum_type),
        )),
        TooltrainDataType::Struct(struct_type) => Ok(TooltrainListDataType::Struct(
            TooltrainTypedListDataType::new(struct_type),
        )),
        _ => Ok(TooltrainListDataType::Generic(Box::new(
            TooltrainGenericListDataType::new(child_type),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::{flexbuffer_coders::TooltrainCoder, parse, types::*};

    #[test]
    fn parses_enum() {
        let result = parse("enum Number<ONE, TWO>").unwrap();
        assert_eq!(result.type_string(), "enum Number<ONE, TWO>");
        let enum_result: TooltrainEnumDataType = result.try_into().unwrap();
        assert_eq!(enum_result.get_name(), "Number");
        assert_eq!(
            enum_result.list_variants().collect::<Vec<&str>>(),
            vec!["ONE", "TWO"]
        );
    }

    #[test]
    fn parses_boolean_list() {
        let result = parse("list<boolean>").unwrap();
        assert_eq!(result.type_string(), "list<boolean>");
        let generic_list_data_type: TooltrainListDataType = result.try_into().unwrap();
        let boolean_list_data_type: TooltrainTypedListDataType<TooltrainBooleanDataType> =
            generic_list_data_type.try_into().unwrap();

        let encoded = boolean_list_data_type
            .encode(vec![true, false, true])
            .unwrap();
        let decoded = boolean_list_data_type.decode(&encoded).unwrap();
        assert_eq!(decoded, vec![true, false, true]);
    }
}

use anyhow::{anyhow, Error};
use pest::{iterators::Pairs, Parser};
use pest_derive::Parser;

mod flexbuffer_coders;
pub mod types;

pub use flexbuffer_coders::CommanderCoder;
pub use types::*;

#[derive(Parser)]
#[grammar = "../../wit/types.pest"] // relative to src
struct TypeParser;

pub fn parse(input: &str) -> Result<CommanderDataType, Error> {
    let pairs = TypeParser::parse(Rule::r#type, input)?;
    expand_type(pairs)
}

fn expand_type(mut pairs: Pairs<'_, Rule>) -> Result<CommanderDataType, Error> {
    match pairs.peek().ok_or(anyhow!("No type found"))?.as_rule() {
        Rule::trigger => {
            pairs.next().unwrap();
            Ok(CommanderTriggerDataType {}.into())
        }
        _ => expand_static_type(pairs),
    }
}

fn expand_static_type(mut pairs: Pairs<'_, Rule>) -> Result<CommanderDataType, Error> {
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

fn expand_primitive_type(mut pairs: Pairs<'_, Rule>) -> Result<CommanderDataType, Error> {
    match pairs
        .next()
        .ok_or(anyhow!("No primitive_type found"))?
        .as_rule()
    {
        Rule::boolean => Ok(CommanderBooleanDataType {}.into()),
        Rule::number => Ok(CommanderNumberDataType {}.into()),
        Rule::string => Ok(CommanderStringDataType {}.into()),
        Rule::bytes => Ok(CommanderBytesDataType {}.into()),
        Rule::color => Ok(CommanderColorDataType {}.into()),
        Rule::path => Ok(CommanderPathDataType {}.into()),
        Rule::url => todo!(),
        Rule::json => Ok(CommanderJsonDataType {}.into()),
        Rule::svg => Ok(CommanderSvgDataType {}.into()),
        _ => unreachable!(),
    }
}

fn expand_enum_type(mut pairs: Pairs<'_, Rule>) -> Result<CommanderEnumDataType, Error> {
    let type_name_pair = pairs.next().unwrap();
    assert_eq!(Rule::type_name, type_name_pair.as_rule());
    let type_name = type_name_pair.as_str().to_string();

    let mut variants: Vec<String> = vec![];
    while let Some(Rule::enum_variant) = pairs.peek().map(|pair| pair.as_rule()) {
        variants.push(pairs.next().unwrap().as_str().to_string());
    }

    Ok(CommanderEnumDataType::new(type_name, variants))
}

fn expand_list_type(pairs: Pairs<'_, Rule>) -> Result<CommanderListDataType, Error> {
    let child_type = expand_static_type(pairs)?;
    match child_type {
        CommanderDataType::Boolean(boolean_type) => Ok(CommanderListDataType::Boolean(
            CommanderTypedListDataType::new(boolean_type),
        )),
        CommanderDataType::Number(number_type) => Ok(CommanderListDataType::Number(
            CommanderTypedListDataType::new(number_type),
        )),
        CommanderDataType::String(string_type) => Ok(CommanderListDataType::String(
            CommanderTypedListDataType::new(string_type),
        )),
        CommanderDataType::Bytes(bytes_type) => Ok(CommanderListDataType::Bytes(
            CommanderTypedListDataType::new(bytes_type),
        )),
        CommanderDataType::Color(color_type) => Ok(CommanderListDataType::Color(
            CommanderTypedListDataType::new(color_type),
        )),
        CommanderDataType::Json(json_type) => Ok(CommanderListDataType::Json(
            CommanderTypedListDataType::new(json_type),
        )),
        CommanderDataType::Svg(svg_type) => Ok(CommanderListDataType::Svg(
            CommanderTypedListDataType::new(svg_type),
        )),
        CommanderDataType::Path(path_type) => Ok(CommanderListDataType::Path(
            CommanderTypedListDataType::new(path_type),
        )),
        CommanderDataType::Enum(enum_type) => Ok(CommanderListDataType::Enum(
            CommanderTypedListDataType::new(enum_type),
        )),
        CommanderDataType::Struct(struct_type) => Ok(CommanderListDataType::Struct(
            CommanderTypedListDataType::new(struct_type),
        )),
        _ => Ok(CommanderListDataType::Generic(Box::new(
            CommanderGenericListDataType::new(child_type),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::{flexbuffer_coders::CommanderCoder, parse, types::*};

    #[test]
    fn parses_enum() {
        let result = parse("enum Number<ONE, TWO>").unwrap();
        assert_eq!(result.type_string(), "enum Number<ONE, TWO>");
        let enum_result: CommanderEnumDataType = result.try_into().unwrap();
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
        let generic_list_data_type: CommanderListDataType = result.try_into().unwrap();
        let boolean_list_data_type: CommanderTypedListDataType<CommanderBooleanDataType> =
            generic_list_data_type.try_into().unwrap();

        let encoded = boolean_list_data_type
            .encode(vec![true, false, true])
            .unwrap();
        let decoded = boolean_list_data_type.decode(&encoded).unwrap();
        assert_eq!(decoded, vec![true, false, true]);
    }
}

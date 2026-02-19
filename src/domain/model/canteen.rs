use strum_macros::{AsRefStr, Display, EnumCount, EnumIter};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, EnumIter, AsRefStr, EnumCount, Hash)]
pub enum Canteen {
    #[strum(serialize = "Academica")]
    Academica,
    #[strum(serialize = "Ahornstraße")]
    Ahorn,
    Bayernallee,
    #[strum(serialize = "Bistro Templergraben")]
    Bistro,
    #[strum(serialize = "Eupener Straße")]
    Eupener,
    Jülich,
    KMAC,
    #[strum(serialize = "Südpark")]
    Süd,
    Vita,
}

impl Canteen {
    pub fn parser() -> parser::CanteenParser {
        parser::CanteenParser
    }
}

pub(super) mod parser {
    use nom::{
        branch::alt,
        bytes::complete::{tag, tag_no_case},
        character::complete::{space0, space1},
        combinator::{eof, opt, recognize},
        sequence::{terminated, tuple},
        IResult,
    };

    use super::Canteen;

    type ParseResult<'a> = IResult<&'a str, Canteen>;

    pub struct CanteenParser;

    impl CanteenParser {
        pub fn parse<'a>(&self, input: &'a str) -> ParseResult<'a> {
            parse(input)
        }
    }

    pub fn parse(input: &str) -> ParseResult<'_> {
        let (input, _) = opt(tuple((tag_no_case("mensa"), space1)))(input)?;

        // Make sure the end of the name is a word boundary, i.e. either whitespace or the end of the string
        let result = terminated(
            alt((
                parse_academica,
                parse_ahorn,
                parse_bayernallee,
                parse_bistro,
                parse_eupener,
                parse_jülich,
                parse_kmac,
                parse_südpark,
                parse_vita,
            )),
            // termination sequence
            alt((space1, eof)),
        )(input)?;

        Ok(result)
    }

    fn parse_academica(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("ac")(input)?;
        let (input, _) = alt((tag_no_case("a"), tag_no_case("er")))(input)?;
        let (input, _) = opt(tuple((
            tag_no_case("demic"),
            alt((tag_no_case("a"), tag_no_case("er"))),
        )))(input)?;

        Ok((input, Canteen::Academica))
    }

    fn parse_ahorn(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((
            recognize(tuple((
                tag_no_case("ahorn"),
                opt(tuple((
                    tag_no_case("stra"),
                    alt((tag("ß"), tag_no_case("ss"))),
                    tag_no_case("e"),
                ))),
            ))),
            recognize(tuple((
                tag_no_case("info"),
                opt(tag_no_case("rmatik")),
                opt(tag_no_case("zentrum")),
            ))),
            tag_no_case("iz"),
        ))(input)?;

        Ok((input, Canteen::Ahorn))
    }

    fn parse_bayernallee(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("bayernallee")(input)?;

        Ok((input, Canteen::Bayernallee))
    }

    fn parse_bistro(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((
            recognize(tuple((tag_no_case("super"), space0, tag_no_case("C")))),
            recognize(tuple((
                tag_no_case("bistro"),
                opt(tuple((space1, tag_no_case("templergraben")))),
            ))),
        ))(input)?;

        Ok((input, Canteen::Bistro))
    }

    fn parse_eupener(input: &str) -> ParseResult<'_> {
        let (input, _) = tuple((
            tag_no_case("eupener"),
            opt(tuple((
                space1,
                tag_no_case("stra"),
                alt((tag("ß"), tag_no_case("ss"))),
                tag_no_case("e"),
            ))),
        ))(input)?;

        Ok((input, Canteen::Eupener))
    }

    fn parse_jülich(input: &str) -> ParseResult<'_> {
        let (input, _) = tag_no_case("jülich")(input)?;

        Ok((input, Canteen::Jülich))
    }

    fn parse_kmac(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((
            tag_no_case("kmac"),
            recognize(tuple((
                tag_no_case("k"),
                alt((tag("-"), space0)),
                tag_no_case("mag"),
            ))),
            recognize(tuple((
                tag_no_case("Kevin"),
                space1,
                tag_no_case("Magnussen"),
            ))),
        ))(input)?;

        Ok((input, Canteen::KMAC))
    }

    fn parse_südpark(input: &str) -> ParseResult<'_> {
        let (input, _) = tuple((tag_no_case("süd"), opt(tag_no_case("park"))))(input)?;

        Ok((input, Canteen::Süd))
    }

    fn parse_vita(input: &str) -> ParseResult<'_> {
        let (input, _) = alt((
            recognize(tuple((
                tag_no_case("vit"),
                alt((tag_no_case("a"), tag_no_case("er"))),
            ))),
            tag_no_case("melaten"),
        ))(input)?;

        Ok((input, Canteen::Vita))
    }
}

impl phf::PhfHash for Canteen {
    fn phf_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().phf_hash(state)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn it_parses_academica() {}
}

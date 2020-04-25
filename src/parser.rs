use nom::number::streaming::le_u32;
use nom::sequence::tuple;
use nom::IResult;

pub fn server_greeting(i: &[u8]) -> IResult<&[u8], (u32, u32)> {
    let (input, (server_version, salt)) = tuple((le_u32, le_u32))(i)?;
    Ok((input, (server_version, salt)))
}

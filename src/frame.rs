type AnyResult<T = ()> = anyhow::Result<T>;

#[derive(Debug)]
pub(crate) enum DataType {
    End,
    Data,
    Metadata,
}

impl TryFrom<u8> for DataType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> AnyResult<Self> {
        match value {
            0 => Ok(Self::End),
            1 => Ok(Self::Data),
            2 => Ok(Self::Metadata),
            _ => Err(anyhow::anyhow!("Invalid DataFrameType")),
        }
    }
}

impl From<DataType> for u8 {
    fn from(value: DataType) -> Self {
        match value {
            DataType::End => 0,
            DataType::Data => 1,
            DataType::Metadata => 2,
        }
    }
}

// TODO: Impl tokio_util::codec for DataFrame

/// MTU is 1500 bytes.
///
/// Frame format:
/// | Len |     Position     | Frame Type |    Data    |
/// |     | Index | Overflow |            |            |
/// |  2  |    2  |     1    |      1     |   n<=1490  |
///
/// Overflow means re-start at 0 (u16::MAX --[+1]-> 0).
/// So if we have to transfer a big file and run out of u16 indexs,
/// then we can use the `overflow` to continue the transfer.
/// The position bytes will be:
/// (65535, false) -> (0, true) -> (1, false) -> ...
#[derive(Debug)]
pub(crate) struct DataFrame {
    data: Vec<u8>,
    position: (u16, bool),
    frame_type: DataType,
}

impl DataFrame {
    pub(crate) fn new(
        data: Vec<u8>,
        position: (u16, bool),
        frame_type: DataType,
    ) -> AnyResult<Self> {
        if data.len() > 1490 {
            return Err(anyhow::anyhow!("Data too large"));
        }

        Ok(Self {
            data,
            position,
            frame_type,
        })
    }
}

fn u8_array_to_u16(array: [u8; 2]) -> u16 {
    ((array[0] as u16) << 8) | array[1] as u16
}

fn u16_to_u8_array(value: u16) -> [u8; 2] {
    [(value >> 8) as u8, value as u8]
}

impl From<DataFrame> for Vec<u8> {
    fn from(value: DataFrame) -> Self {
        let mut result: Vec<u8> = Vec::new();

        let len = u16_to_u8_array(value.data.len() as u16);

        result.extend(len);
        result.extend(u16_to_u8_array(value.position.0));
        result.push(value.frame_type.into());
        result.extend(value.data);

        result
    }
}

impl TryFrom<Vec<u8>> for DataFrame {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> AnyResult<Self> {
        let position_idex = u8_array_to_u16([value[2], value[3]]);
        let position_overflow = value[4] == 1;
        let frame_type = value[5].try_into()?;
        let data = value[6..].to_vec();

        Ok(Self {
            data,
            position: (position_idex, position_overflow),
            frame_type,
        })
    }
}

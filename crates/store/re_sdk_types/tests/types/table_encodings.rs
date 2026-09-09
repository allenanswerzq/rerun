use re_sdk_types::encodings::{TableBuffer, TableValue, TensorBuffer, TensorData};
use re_types_core::{FromArrow as _, FromArrowOpt as _, ToArrow as _, ToArrowOpt as _};

#[test]
fn table_value_roundtrip() {
    let expected = [
        Some(TableValue::Number(42.5)),
        Some(TableValue::Text("AAPL".into())),
        None,
        Some(TableValue::Text("".into())),
        Some(TableValue::Number(-1.0)),
    ];
    let array = TableValue::to_arrow_opt(expected.iter().map(Option::as_ref)).unwrap();
    assert_eq!(TableValue::from_arrow_opt(&array).unwrap(), expected);
    assert!(TableValue::from_arrow(&array).is_err());

    let values: Vec<_> = expected.into_iter().flatten().collect();
    let array = TableValue::to_arrow(&values).unwrap();
    assert_eq!(TableValue::from_arrow(&array).unwrap(), values);
}

#[test]
fn table_buffer_roundtrip() {
    let expected = [
        Some(TableBuffer::Numeric(TensorData::new(
            vec![2, 2],
            TensorBuffer::U64(vec![0, 1, u64::MAX - 1, u64::MAX].into()),
        ))),
        None,
        Some(TableBuffer::Text(vec!["AAPL".into(), "".into()])),
        Some(TableBuffer::Text(Vec::new())),
        Some(TableBuffer::Numeric(TensorData::from(vec![1.5_f64, -2.0]))),
    ];
    let array = TableBuffer::to_arrow_opt(expected.iter().map(Option::as_ref)).unwrap();
    assert_eq!(TableBuffer::from_arrow_opt(&array).unwrap(), expected);
    assert!(TableBuffer::from_arrow(&array).is_err());

    let values: Vec<_> = expected.into_iter().flatten().collect();
    let array = TableBuffer::to_arrow(&values).unwrap();
    assert_eq!(TableBuffer::from_arrow(&array).unwrap(), values);
}

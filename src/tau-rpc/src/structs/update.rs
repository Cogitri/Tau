use serde::{Deserialize, Deserializer};

use crate::Operation;
use crate::ViewId;

#[derive(Debug, PartialEq, Clone)]
pub struct Update {
    pub rev: Option<u64>,
    pub operations: Vec<Operation>,
    pub pristine: bool,
    pub view_id: ViewId,
}

#[derive(Deserialize, Debug, PartialEq)]
struct InnerUpdate {
    pub rev: Option<u64>,
    #[serde(rename = "ops")]
    pub operations: Vec<Operation>,
    pub pristine: bool,
}

#[derive(Deserialize, Debug, PartialEq)]
struct UpdateHelper {
    pub update: InnerUpdate,
    pub view_id: ViewId,
}

impl<'de> Deserialize<'de> for Update {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|UpdateHelper { update, view_id }| Update {
            rev: update.rev,
            operations: update.operations,
            pristine: update.pristine,
            view_id,
        })
    }
}

#[test]
fn deserialize_update() {
    use serde_json;
    use std::str::FromStr;

    use super::operation::{Operation, OperationType};
    use super::Line;

    let s = r#"{"update":{"ops":[{"n":60,"op":"invalidate"},{"lines":[{"cursor":[0],"styles":[],"text":"Bar"},{"styles":[],"text":"Foo"}],"n":12,"op":"ins"}],"pristine":true},"view_id":"view-id-1"}"#;
    let update = Update {
        operations: vec![
            Operation {
                operation_type: OperationType::Invalidate,
                nb_lines: 60,
                line_num: None,
                lines: vec![],
            },
            Operation {
                operation_type: OperationType::Insert,
                nb_lines: 12,
                line_num: None,
                lines: vec![
                    Line {
                        cursor: vec![0],
                        styles: vec![],
                        text: "Bar".to_owned(),
                        line_num: None,
                    },
                    Line {
                        cursor: vec![],
                        styles: vec![],
                        text: "Foo".to_owned(),
                        line_num: None,
                    },
                ],
            },
        ],
        pristine: true,
        rev: None,
        view_id: FromStr::from_str("view-id-1").unwrap(),
    };
    let deserialized: Result<Update, _> = serde_json::from_str(s);
    assert_eq!(deserialized.unwrap(), update);
}

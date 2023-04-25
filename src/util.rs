use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct UntaggedResult<T, E>(
    #[serde(deserialize_with = "untagged_result::deserialize")] pub Result<T, E>,
)
where
    T: for<'d> Deserialize<'d>,
    E: for<'d> Deserialize<'d>;

mod untagged_result {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum UntaggedResult<T, E> {
        Ok(T),
        Err(E),
    }

    pub fn serialize<S, T, E>(result: &Result<T, E>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
        E: Serialize,
    {
        match result {
            Ok(ok) => ok.serialize(serializer),
            Err(err) => err.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D, T, E>(deserializer: D) -> Result<Result<T, E>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
        E: Deserialize<'de>,
    {
        match UntaggedResult::deserialize(deserializer) {
            Ok(UntaggedResult::Ok(ok)) => Ok(Ok(ok)),
            Ok(UntaggedResult::Err(err)) => Ok(Err(err)),
            Err(err) => Err(err),
        }
    }
}
